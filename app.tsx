import Bar from "./src/app/Bar"
import app from "ags/gtk4/app"
import style from "./style.scss"
import DesktopScriptsLib from "./src/lib/ExternalCommand"
import {GlobalRequestHandler} from "./src/requesthandler/GlobalRequestHandler";
import {createBinding, For, This} from "ags"
import Notifications from "./src/app/Notifications";
import {customExecutableDependencies, dependencies} from "./src/lib/Dependency";
import BluetoothManager from "./src/app/BluetoothManager";

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

        BluetoothManager()

        Notifications()

        return (
            <For each={monitors}>
                {(monitor) => (
                    <This this={app}>
                        <Bar gdkmonitor={monitor}/>
                    </This>
                )}
            </For>
        )
    },
})
