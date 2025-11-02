import {Gtk} from "ags/gtk4"
import {interval} from "ags/time"
import {createState} from "ags"
import DesktopScriptLib from "../../../../lib/ExternalCommand"

export default function AirplaneModeButtonQS(
    {minWidth}: { minWidth: number },
) {
    const [mode, setMode] = createState<string>("up")
    const [icon, setIcon] = createState<string>("airplane-mode-disabled-symbolic");

    function updateAirplaneModeState() {
        DesktopScriptLib.execAsync("airplane_mode status")
            .then(output => {
                setMode(output.trim())
                if (output.trim() == "up") {
                    setIcon("airplane-mode-disabled-symbolic")
                } else {
                    setIcon("airplane-mode-symbolic")
                }
            })
            .catch((err) => printerr(err));
    }

    interval(1000, () => {
        updateAirplaneModeState();
    });

    updateAirplaneModeState();

    return (
        <togglebutton
            css={`
                min-width: ${minWidth}px;
            `}
            active={mode.get() == "down"}
            onClicked={async () => {
                await DesktopScriptLib.execAsync("airplane_mode toggle")
                    .then(_ => {
                        updateAirplaneModeState()
                    })
                    .catch((err) => printerr(err));
            }}
        >

            <box spacing={8}>
                <image
                    iconName={icon}
                    iconSize={Gtk.IconSize.NORMAL}
                />
                <label label={"Airplane mode"}/>
            </box>
        </togglebutton>
    );
}
