import {Gtk} from "ags/gtk4"
import {interval} from "ags/time"
import {createState} from "ags"
import {shAsync} from "../../../../lib/ExternalCommand"

export default function DarkModeButtonQS(
    {minWidth}: { minWidth: number },
) {
    const [mode, setMode] = createState<string>("prefer-dark")

    function updateDarkModeState() {
        shAsync("gsettings get org.gnome.desktop.interface color-scheme")
            .then(output => {
                setMode(output.trim().replaceAll("'", ""))
            })
            .catch((err) => printerr(err))
    }

    interval(1000, () => {
        updateDarkModeState()
    })

    updateDarkModeState()

    return (
        <togglebutton
            css={`
                min-width: ${minWidth}px;
            `}
            active={mode.get() == "prefer-dark"}
            onClicked={async () => {
                await shAsync(`gsettings set org.gnome.desktop.interface color-scheme ${mode.get() == "prefer-dark" ? "prefer-light" : "prefer-dark"}`)
                    .then(_ => {
                        updateDarkModeState()
                    })
                    .catch((err) => printerr(err));
                await shAsync(`kvantummanager --set ${mode.get() == "prefer-dark" ? "KvLibadwaita" : "KvLibadwaitaDark"}`)
                    .catch((err) => printerr(err));
            }}
        >
            <box spacing={8}>
                <image
                    iconName={"night-light-symbolic"}
                    iconSize={Gtk.IconSize.NORMAL}
                />
                <label label={"Dark mode"}/>
            </box>
        </togglebutton>
    );
}
