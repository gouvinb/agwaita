import {execAsync, subprocess} from "ags/process"
import {WM, Workspace, WorkspaceEvent} from "../WM"
import GObject, {getter, register} from "gnim/gobject"
import GLib from "gi://GLib"
import {Accessor} from "gnim"
import {Log} from "../../../lib/Logger"

interface NiriRawWorkspace {
    id: number,
    idx: number,
    name: string,
    output: string,
    is_urgent: boolean,
    is_active: boolean,
    is_focused: boolean,
    active_window_id: number
}

let instance: Niri

@register({GTypeName: "Niri"})
export class Niri extends GObject.Object implements WM {
    declare $signals: GObject.Object.SignalSignatures & {
        "notify::workspaces": () => void
        "notify::workspace-event": () => void
        "workspace-event": (value: number) => void
    }

    #workspaces: Workspace[] = []
    #switchDebounceId: number | null = null
    #switchPendingIndex: number | null = null
    #switchDebounceDelay = 50
    #workspaceEvent: number = WorkspaceEvent.Unsupported

    constructor() {
        super()
        const proc = subprocess(
            "niri msg -j event-stream",
            (_) => {
            },
            (stderr) => {
                Log.e("Niri service", stderr)
            },
        )

        proc.connect("stdout", (_, rawJSON: string) => {
            if (rawJSON == undefined) {
                this.#setWorkspaceEvent(WorkspaceEvent.Unsupported)
                return
            }
            const event = JSON.parse(rawJSON) as Record<string, object>
            const eventName = Object.keys(event)[0]

            switch (eventName) {
                case "WorkspacesChanged":
                    this.#setWorkspaceEvent(WorkspaceEvent.Changed)
                    return
                case "WorkspaceActivated":
                    this.#setWorkspaceEvent(WorkspaceEvent.Activated)
                    return
                case "WorkspaceUrgencyChanged":
                    this.#setWorkspaceEvent(WorkspaceEvent.UrgencyChanged)
                    return
                default:
                    this.#setWorkspaceEvent(WorkspaceEvent.Unsupported)
                    return
            }
        })

        this.#refreshWorkspaces()
            .catch((err) => Log.e("Niri service", `Failed to refresh workspaces`, err))
    }

    @getter(Array)
    get workspaces() {
        return this.#workspaces
    }

    @getter(Number)
    get workspace_event() {
        return this.#workspaceEvent
    }

    static get_default() {
        if (!instance) instance = new Niri()
        return instance
    }

    eventWorkspacesStreams(): Accessor<WorkspaceEvent | null> {
        const listeners: Array<(v: WorkspaceEvent | null) => void> = []
        const cb = (_: Niri) => {
            const ev = this.#workspaceEvent as WorkspaceEvent
            for (const l of listeners) l(ev)
        }
        this.connect("notify::workspace-event", cb as (_: Niri) => void)
        return new Accessor(() => null, (fn: (v: WorkspaceEvent | null) => void) => {
            listeners.push(fn)
            return () => {
                const i = listeners.indexOf(fn)
                if (i >= 0) listeners.splice(i, 1)
            }
        })
    }

    async listsWorkspaces(): Promise<Workspace[]> {
        try {
            await this.#refreshWorkspaces()
        } catch (error) {
            Log.e("Niri service", `Failed to refresh workspaces`, error)
        }
        return this.#workspaces
    }

    switchToWorkspace(index: number): Promise<void> {
        this.#switchPendingIndex = index
        if (this.#switchDebounceId) {
            GLib.source_remove(this.#switchDebounceId)
        }
        return new Promise((resolve, reject) => {
            this.#switchDebounceId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, this.#switchDebounceDelay, () => {
                const i = this.#switchPendingIndex!
                this.#switchDebounceId = null
                this.#switchPendingIndex = null
                execAsync(`niri msg action focus-workspace ${i}`)
                    .then(() => resolve())
                    .catch((e) => {
                        Log.e("Niri service", `Failed to switch to workspace ${i}`, e)
                        reject(e)
                    })
                return GLib.SOURCE_REMOVE
            })
        })
    }

    #setWorkspaceEvent(v: number) {
        if (this.#workspaceEvent === v) return
        this.#workspaceEvent = v
        try {
            this.notify("workspace-event")
        } catch (e) {
            Log.e("Niri service", `Failed to notify workspace-event signal ${v}`, e)
        }
    }

    async #refreshWorkspaces() {
        const rawJSON = await execAsync("niri msg -j workspaces")
        const ws = JSON.parse(rawJSON) as NiriRawWorkspace[]
        this.#workspaces = ws
            .map((w) => ({
                index: w.idx,
                name: w.name != null ? w.name : `${w.idx}`,
                focused: w.is_focused,
                active: w.is_active,
                urgent: w.is_urgent,
                output: w.output,
            }))
            .sort((a, b) => a.index - b.index)
        this.notify("workspaces")
    }
}
