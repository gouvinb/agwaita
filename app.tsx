import {Bar} from "./src/app/Bar"
import app from "ags/gtk4/app"
import style from "./style.scss"
import DesktopScriptsLib from "./src/lib/ExternalCommand"
import {GlobalRequestHandler} from "./src/requesthandler/GlobalRequestHandler";
import {createBinding, For, onCleanup} from "ags"
import {Notifications} from "./src/app/Notifications";
import {customExecutableDependencies, dependencies} from "./src/lib/Dependency";
import BluetoothManager from "./src/app/BluetoothManager";

import AstalBattery from "gi://AstalBattery"
import AstalBluetooth from "gi://AstalBluetooth"
import AstalNotifd from "gi://AstalNotifd"
import AstalWp from "gi://AstalWp"
import PowerProfiles from "gi://AstalPowerProfiles"
import Tray from "gi://AstalTray"
import Agenda from "./src/services/Agenda";
import Brightness from "./src/services/Brightness";
import PowerMenu from "./src/app/PowerMenu";

app.start({
    css: style,
    requestHandler(argv: string[], response: (response: string) => void) {
        const [cmd, arg, ...rest] = argv
        const globalRequestHandler = new GlobalRequestHandler(response);
        try {
            if (cmd == null) throw new Error("No command provided")
            globalRequestHandler.parse(cmd, arg, ...rest)
        } catch (error) {
            globalRequestHandler.help(`${error}`)
        }
    },
    main() {
        if (!dependencies(
            "niri",
            "gsettings",
            "pavucontrol",
            "kvantummanager",
            "systemctl",
            "swaylock",
            "loginctl",
        ) && !customExecutableDependencies(
            DesktopScriptsLib.pathOf("prelock")
        )) {
            app.quit()
        }

        const monitors = createBinding(app, "monitors")

        const notifd = AstalNotifd.get_default()
        const bluetooth = AstalBluetooth.get_default()
        const wp = AstalWp.get_default()
        const battery = AstalBattery.get_default()
        const powerprofiles = PowerProfiles.get_default()
        const tray = Tray.get_default()

        const agenda = Agenda.get_with_signals_initialized()

        const brightness = Brightness.get_default()

        BluetoothManager(bluetooth)

        PowerMenu()

        Notifications(notifd)

        onCleanup(() => {
            agenda.stopAllSignals()
        })

        return For({
            each: monitors,
            children: (gdkmonitor) => Bar({
                gdkmonitor,
                notifd,
                bluetooth,
                wp,
                battery,
                powerprofiles,
                tray,
                agenda,
                brightness,
            }),
        })
    },
})
