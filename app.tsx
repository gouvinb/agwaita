import Bar from "./src/app/Bar"
import app from "ags/gtk4/app"
import style from "./style.scss"
import DesktopScriptsLib from "./src/lib/ExternalCommand"
import {GlobalRequestHandler} from "./src/requesthandler/GlobalRequestHandler";
import {createBinding, For, This} from "ags"
import Notifications from "./src/app/Notifications";
import {customExecutableDependencies, dependencies} from "./src/lib/Dependency";

app.start({
    css: style,
    requestHandler(argv: string[], response: (response: string) => void) {
        const [cmd, arg, ...rest] = argv
        try {
            if (cmd == null) throw new Error("No command provided")
            new GlobalRequestHandler(response).parse(cmd, arg, ...rest)
        } catch (error) {
            response(`${error}`)
        }
    },
    main() {
        if (!dependencies(
            "niri",
            "gsettings",
            "pavucontrol",
            "overskride",
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
