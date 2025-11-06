import {onCleanup} from "ags"
import app from "ags/gtk4/app"
import {Astal, Gdk, Gtk} from "ags/gtk4"
import {Clock} from "../components/bar/Clock/Clock"
import SystemdUnitFailed from "../components/bar/SystemdUnitFailed"
import SysTray from "../components/bar/SysTray"
import {SystemIndicators} from "../components/bar/SystemIndicators/SystemIndicators"
import Workspaces from "../components/bar/Workspaces";
import {Dimensions} from "../lib/ui/Diemensions";

export default function Bar({gdkmonitor}: { gdkmonitor: Gdk.Monitor }) {
    const {TOP, LEFT, RIGHT} = Astal.WindowAnchor

    let win: Astal.Window

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
                padding: ${Dimensions.smallSpacing}px;
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
                        spacing={Dimensions.smallSpacing}
                    >
                        <Workspaces gdkmonitor={gdkmonitor}/>
                    </box>

                    <box
                        $type="center"
                        spacing={Dimensions.smallSpacing}
                    >
                        <Clock
                            popoverRequestHeight={Dimensions.notificationCenterHeight}
                        />
                    </box>

                    <box
                        $type="end"
                        spacing={Dimensions.smallSpacing}
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
