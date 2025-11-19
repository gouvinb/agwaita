import {onCleanup} from "ags"
import app from "ags/gtk4/app"
import {Astal, Gdk, Gtk} from "ags/gtk4"
import {Clock} from "../components/bar/Clock/Clock"
import SystemdUnitFailed from "../components/bar/SystemdUnitFailed"
import SysTray from "../components/bar/SysTray"
import {SystemIndicators} from "../components/bar/SystemIndicators/SystemIndicators"
import Workspaces from "../components/bar/Workspaces";
import {Dimensions} from "../lib/ui/Dimensions";

import AstalBattery from "gi://AstalBattery"
import AstalBluetooth from "gi://AstalBluetooth"
import AstalNotifd from "gi://AstalNotifd"
import AstalWp from "gi://AstalWp"
import PowerProfiles from "gi://AstalPowerProfiles"
import Tray from "gi://AstalTray"
import Agenda from "../services/Agenda";
import Brightness from "../services/Brightness";

interface BarProps {
    gdkmonitor: Gdk.Monitor,
    notifd: AstalNotifd.Notifd,
    bluetooth: AstalBluetooth.Bluetooth,
    wp: AstalWp.Wp,
    battery: AstalBattery.Device,
    powerprofiles: PowerProfiles.PowerProfiles,
    tray: Tray.Tray,
    agenda: Agenda,
    brightness: Brightness,
}

export function Bar(
    {
        gdkmonitor,
        notifd,
        bluetooth,
        wp,
        battery,
        powerprofiles,
        tray,
        agenda,
        brightness,
    }: BarProps
) {
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
                            agenda={agenda}
                            notifd={notifd}
                            popoverRequestHeight={Dimensions.notificationCenterHeight}/>
                    </box>

                    <box
                        $type="end"
                        spacing={Dimensions.smallSpacing}
                    >
                        <box><SystemdUnitFailed/></box>
                        <box><SysTray tray={tray}/></box>
                        <box><SystemIndicators
                            notifd={notifd}
                            bluetooth={bluetooth}
                            wp={wp}
                            battery={battery}
                            powerprofiles={powerprofiles}
                            brightness={brightness}
                        /></box>
                    </box>

                </centerbox>
            </box>
        </window>
    )
}
