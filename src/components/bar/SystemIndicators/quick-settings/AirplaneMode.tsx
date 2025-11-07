import {Gtk} from "ags/gtk4"
import {interval, Timer} from "ags/time"
import {createState} from "ags"
import DesktopScriptLib from "../../../../lib/ExternalCommand"
import {Dimensions} from "../../../../lib/ui/Dimensions";
import {Log} from "../../../../lib/Logger";
import {Lifecycle} from "../../../../lib/Lifecyle";

export function AirplaneModeButtonQS(
    {parentLifeCycle = null, minWidth}: {
        parentLifeCycle?: Lifecycle | null,
        minWidth: number,
    },
) {
    const [mode, setMode] = createState<string>("up")
    const [icon, setIcon] = createState<string>("airplane-mode-disabled-symbolic");

    let airplaneModeStateTimer: Timer | null = null

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
            .catch((err) => Log.e("AirplaneModeButtonQS", `Cannot get airplane mode status`, err))
    }

    if (parentLifeCycle != null) {
        parentLifeCycle.onStart(() => {
            if (airplaneModeStateTimer == null) {
                airplaneModeStateTimer = interval(1000, () => updateAirplaneModeState())
            }
        })
        parentLifeCycle.onStop(() => {
            airplaneModeStateTimer?.cancel()
            airplaneModeStateTimer = null
        })
    }

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
                    .catch((err) => Log.e("AirplaneModeButtonQS", `Cannot toggle airplane mode`, err))
            }}
        >
            <box spacing={Dimensions.normalSpacing}>
                <image
                    iconName={icon}
                    iconSize={Gtk.IconSize.NORMAL}
                />
                <label label={"Airplane mode"}/>
            </box>
        </togglebutton>
    );
}
