import GLib from "gi://GLib"
import {Niri} from "./impl/Niri"
import {Accessor} from "gnim"
import GObject from "gnim/gobject"

export interface WM extends GObject.Object {
    eventWorkspacesStreams(): Accessor<WorkspaceEvent | null>

    listsWorkspaces(): Promise<Workspace[]>

    switchToWorkspace(index: number): Promise<void>
}

export interface Workspace {
    index: number
    name: string
    focused: boolean
    active: boolean
    urgent: boolean
    output: string
}

export enum WorkspaceEvent {
    Unsupported,
    Changed,
    Activated,
    UrgencyChanged
}

export default function currentWM(): WM {
    const desktop = (GLib.getenv("XDG_CURRENT_DESKTOP") || "").toLowerCase()
    switch (desktop) {
        case "niri":
            return Niri.get_default()
        default:
            throw "Unknown WM"
    }
}
