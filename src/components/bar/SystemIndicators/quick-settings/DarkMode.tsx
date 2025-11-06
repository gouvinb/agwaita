import {Gtk} from "ags/gtk4"
import {interval} from "ags/time"
import {createState} from "ags"
import {shAsync} from "../../../../lib/ExternalCommand"
import {Dimensions} from "../../../../lib/ui/Diemensions";
import {Log} from "../../../../lib/Logger";

export default function DarkModeButtonQS(
    {minWidth}: { minWidth: number },
) {
    const [mode, setMode] = createState<string>("prefer-dark")

    function updateDarkModeState() {
        shAsync("gsettings get org.gnome.desktop.interface color-scheme")
            .then(output => {
                setMode(output.trim().replaceAll("'", ""))
            })
            .catch((err) => Log.e("DarkModeButtonQS", `Cannot get color scheme`, err))
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
                const adwColorScheme = mode.get() == "prefer-dark" ? "prefer-light" : "prefer-dark"
                const kvColorScheme = mode.get() == "prefer-dark" ? "KvLibadwaita" : "KvLibadwaitaDark"

                await shAsync(`gsettings set org.gnome.desktop.interface color-scheme ${adwColorScheme}`)
                    .then(_ => updateDarkModeState())
                    .catch((err) => Log.e("DarkModeButtonQS", `Cannot set ${adwColorScheme} color scheme`, err))
                await shAsync(`kvantummanager --set ${kvColorScheme}`)
                    .catch((err) => Log.e("DarkModeButtonQS", `Cannot set ${kvColorScheme} color scheme`, err))
            }}
        >
            <box spacing={Dimensions.normalSpacing}>
                <image
                    iconName={"night-light-symbolic"}
                    iconSize={Gtk.IconSize.NORMAL}
                />
                <label label={"Dark mode"}/>
            </box>
        </togglebutton>
    );
}
