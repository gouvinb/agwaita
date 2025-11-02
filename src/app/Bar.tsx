import {onCleanup} from "ags"
import app from "ags/gtk4/app"
import {Astal, Gdk, Gtk} from "ags/gtk4"
import {Clock} from "../components/bar/Clock/Clock"
import SystemdUnitFailed from "../components/bar/SystemdUnitFailed"
import SysTray from "../components/bar/SysTray"
import {SystemIndicators} from "../components/bar/SystemIndicators/SystemIndicators"
import Workspaces from "../components/bar/Workspaces";

export default function Bar({gdkmonitor}: { gdkmonitor: Gdk.Monitor }) {
    let win: Astal.Window
    const {TOP, LEFT, RIGHT} = Astal.WindowAnchor

    onCleanup(() => {
        win.destroy()
    })

    return (
        <window
            $={(self) => (win = self)}
            name="bar"
            namespace="ags-bar"
            visible
            css={`
                padding: 4px 8px;
            `}
            cssClasses={["ags-bar"]}
            gdkmonitor={gdkmonitor}
            exclusivity={Astal.Exclusivity.EXCLUSIVE}
            anchor={TOP | LEFT | RIGHT}
            application={app}
        >
            <box
                orientation={Gtk.Orientation.VERTICAL}>
                <centerbox>

                    <box
                        $type="start"
                        spacing={4}
                    >
                        <Workspaces gdkmonitor={gdkmonitor}/>
                    </box>

                    <box
                        $type="center"
                        spacing={4}
                    >
                        <Clock
                            popoverRequestHeight={640}
                        />
                    </box>

                    <box
                        $type="end"
                        spacing={4}
                    >
                        <SystemdUnitFailed/>
                        <SysTray/>
                        <SystemIndicators/>
                    </box>

                </centerbox>
            </box>
        </window>
    )
}
