
use ort::session::Session;
use serde_json::Value as Json;
use std::path::Path;
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer, TruncationParams};

const LONGUEUR_SEQUENCE: usize = 32;
const SEUIL_ABSOLU: f32 = 0.55;
const ECART_MAX_AU_MEILLEUR: f32 = 0.25;

pub struct Concept {
    pub symbole: String,
    pub est_nerd_font: bool,
    embedding: Vec<f32>,
}

pub struct IndexSemantique {
    session: Session,
    tokenizer: Tokenizer,
    concepts: Vec<Concept>,
}

impl IndexSemantique {
    pub fn charger(dossier: &Path) -> Result<Self, String> {
        let mut tokenizer = Tokenizer::from_file(dossier.join("tokenizer.json"))
            .map_err(|e| format!("chargement tokenizer: {e}"))?;
        tokenizer
            .with_truncation(Some(TruncationParams {
                max_length: LONGUEUR_SEQUENCE,
                ..Default::default()
            }))
            .map_err(|e| e.to_string())?;
        tokenizer.with_padding(Some(PaddingParams {
            strategy: PaddingStrategy::Fixed(LONGUEUR_SEQUENCE),
            ..Default::default()
        }));

        let session = Session::builder()
            .map_err(|e| e.to_string())?
            .commit_from_file(dossier.join("model.onnx"))
            .map_err(|e| format!("chargement model.onnx: {e}"))?;

        let metadata_texte = std::fs::read_to_string(dossier.join("metadata.json"))
            .map_err(|e| e.to_string())?;
        let metadata: Json = serde_json::from_str(&metadata_texte).map_err(|e| e.to_string())?;
        let concepts_json = metadata
            .get("concepts")
            .and_then(Json::as_array)
            .ok_or("metadata.json: champ 'concepts' manquant")?;

        let mut symboles = Vec::with_capacity(concepts_json.len());
        let mut descriptions = Vec::with_capacity(concepts_json.len());

        for c in concepts_json {
            let variants = c.get("variants");
            let emoji = variants
                .and_then(|v| v.get("emoji"))
                .and_then(Json::as_array)
                .and_then(|a| a.first())
                .and_then(Json::as_str);
            let nerd_font = variants
                .and_then(|v| v.get("nerd_font"))
                .and_then(|v| v.get("symbols"))
                .and_then(Json::as_array)
                .and_then(|a| a.first())
                .and_then(Json::as_str);

            let est_nerd_font = emoji.is_none() && nerd_font.is_some();
            let symbole = emoji.or(nerd_font).unwrap_or("❓").to_string();

            let alias = |cle: &str| -> Vec<String> {
                c.get(cle)
                    .and_then(Json::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Json::as_str)
                    .map(str::to_string)
                    .collect()
            };
            let mut termes = alias("aliases_fr");
            termes.extend(alias("aliases_en"));

            symboles.push((symbole, est_nerd_font));
            descriptions.push(termes.join(" ; "));
        }

        let mut index = Self {
            session,
            tokenizer,
            concepts: Vec::with_capacity(symboles.len()),
        };
        let embeddings = index.encoder_lot(&descriptions)?;
        index.concepts = symboles
            .into_iter()
            .zip(embeddings)
            .map(|((symbole, est_nerd_font), embedding)| Concept { symbole, est_nerd_font, embedding })
            .collect();

        Ok(index)
    }

    fn encoder(&mut self, texte: &str) -> Result<Vec<f32>, String> {
        Ok(self.encoder_lot(std::slice::from_ref(&texte.to_string()))?.remove(0))
    }

    fn encoder_lot(&mut self, textes: &[String]) -> Result<Vec<Vec<f32>>, String> {
        let encodages = self
            .tokenizer
            .encode_batch(textes.to_vec(), true)
            .map_err(|e| e.to_string())?;

        let n = LONGUEUR_SEQUENCE;
        let lot = encodages.len();
        let mut ids = Vec::with_capacity(lot * n);
        let mut masque = Vec::with_capacity(lot * n);
        let mut types = Vec::with_capacity(lot * n);
        for e in &encodages {
            ids.extend(e.get_ids().iter().map(|&x| x as i64));
            masque.extend(e.get_attention_mask().iter().map(|&x| x as i64));
            types.extend(e.get_type_ids().iter().map(|&x| x as i64));
        }

        let entree_ids = ort::value::Tensor::from_array(([lot, n], ids)).map_err(|e| e.to_string())?;
        let entree_masque = ort::value::Tensor::from_array(([lot, n], masque)).map_err(|e| e.to_string())?;
        let entree_types = ort::value::Tensor::from_array(([lot, n], types)).map_err(|e| e.to_string())?;

        let sorties = self
            .session
            .run(ort::inputs![
                "input_ids" => entree_ids,
                "attention_mask" => entree_masque,
                "token_type_ids" => entree_types,
            ])
            .map_err(|e| e.to_string())?;

        let (forme, donnees) = sorties["embeddings"]
            .try_extract_tensor::<f32>()
            .map_err(|e| e.to_string())?;
        let dims = *forme.last().ok_or("sortie du modèle vide")? as usize;
        Ok(donnees.chunks(dims).map(<[f32]>::to_vec).collect())
    }

    pub fn rechercher(&mut self, requete: &str, k: usize, inclure_nerd_font: bool) -> Vec<String> {
        let Ok(embedding_requete) = self.encoder(requete) else {
            return Vec::new();
        };

        let mut classement: Vec<(usize, f32)> = self
            .concepts
            .iter()
            .enumerate()
            .filter(|(_, c)| inclure_nerd_font || !c.est_nerd_font)
            .map(|(idx, c)| {
                let score: f32 = c.embedding.iter().zip(&embedding_requete).map(|(a, b)| a * b).sum();
                (idx, score)
            })
            .collect();
        classement.sort_by(|a, b| b.1.total_cmp(&a.1));

        let Some(&(_, meilleur)) = classement.first() else {
            return Vec::new();
        };
        let seuil = SEUIL_ABSOLU.max(meilleur - ECART_MAX_AU_MEILLEUR);

        classement
            .into_iter()
            .take_while(|(_, score)| *score >= seuil)
            .take(k)
            .map(|(idx, _)| self.concepts[idx].symbole.clone())
            .collect()
    }
}
