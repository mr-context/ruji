// Extension GNOME Shell minimale : expose la position globale du curseur via D-Bus
// (org.ruji.Pointer, méthode GetPosition -> (x, y)). Le compositeur (Mutter) connaît
// toujours la vraie position du pointeur, quelle que soit la fenêtre survolée — c'est
// ce que X11/XQueryPointer ne peut pas garantir au-dessus d'une surface Wayland native.

import Gio from 'gi://Gio';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

const IFACE_XML = `
<node>
  <interface name="org.ruji.Pointer">
    <method name="GetPosition">
      <arg type="i" direction="out" name="x"/>
      <arg type="i" direction="out" name="y"/>
    </method>
  </interface>
</node>`;

const PointerService = class {
    constructor() {
        this._dbusImpl = Gio.DBusExportedObject.wrapJSObject(IFACE_XML, this);
    }

    GetPosition() {
        const [x, y] = global.get_pointer();
        return [x, y];
    }

    export(connection, path) {
        this._dbusImpl.export(connection, path);
    }

    unexport() {
        this._dbusImpl.unexport();
    }
};

export default class RujiPointerExtension extends Extension {
    enable() {
        this._service = new PointerService();
        this._ownerId = Gio.bus_own_name(
            Gio.BusType.SESSION,
            'org.ruji.Pointer',
            Gio.BusNameOwnerFlags.NONE,
            (connection) => this._service.export(connection, '/org/ruji/Pointer'),
            null,
            null,
        );
    }

    disable() {
        if (this._ownerId) {
            Gio.bus_unown_name(this._ownerId);
            this._ownerId = null;
        }
        if (this._service) {
            this._service.unexport();
            this._service = null;
        }
    }
}
