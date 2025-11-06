import {Gdk, Gtk} from "ags/gtk4"
import {createState, For} from "ags"
import currentWM, {WM, Workspace} from "../../services/wm/WM"
import {Log} from "../../lib/Logger";
import {Dimensions} from "../../lib/ui/Diemensions";

type WorkspaceUi = {
    index: number,
    name: string,
    urgent: boolean,
    focused: boolean,
    active: boolean,
    classes: string[],
}

export default function Workspaces({gdkmonitor}: { gdkmonitor: Gdk.Monitor }) {
    const wm = currentWM()

    const [workspaces, setWorkspaces] = createState<WorkspaceUi[]>([])

    if ("connect" in wm) {
        const workspaceEventSignalId = (wm as WM).connect("notify::workspace-event", () => updateWorkspaces())
        Log.i("Workspaces", `Connected to workspace event signal with id: ${workspaceEventSignalId}`)
    }

    function updateWorkspaces() {
        wm.listsWorkspaces()
            .then((ws: Workspace[]) => {
                setWorkspaces(
                    ws
                        .filter(w => w.output === gdkmonitor.get_connector())
                        .sort((a, b) => a.index - b.index)
                        .map(w => ({
                            index: w.index,
                            name: w.name,
                            urgent: w.urgent,
                            focused: w.focused,
                            active: w.active,
                            classes: [],//resolveWorkspaceClasses(w)
                        }))
                )
            })
            .catch((err) => Log.e("Workspaces", `Failed to list workspaces`, err))
    }

    function resolveWorkspaceClasses(
        urgent: boolean,
        focused: boolean,
        active: boolean,
    ) {
        const classes = ["workspace"]
        if (urgent) classes.push("urgent")
        else if (focused) classes.push("focused")
        else if (active) classes.push("active")
        return classes
    }

    updateWorkspaces()


    return (
        <box
            cssClasses={["workspaces"]}
            spacing={Dimensions.smallSpacing}
            halign={Gtk.Align.START}
        >
            <For each={workspaces}>
                {(workspaceUi) => <button
                    css={`
                        padding: 0 10px;
                    `}
                    cssClasses={resolveWorkspaceClasses(workspaceUi.urgent, workspaceUi.focused, workspaceUi.active)}
                    onClicked={() => {
                        wm.switchToWorkspace(workspaceUi.index)
                            .then(() => updateWorkspaces())
                            .catch((err: unknown) => Log.e("Workspaces", `Failed to switch to workspace ${workspaceUi.index}`, err))
                    }}
                    label={workspaceUi.name}
                />}
            </For>
        </box>
    )
}
