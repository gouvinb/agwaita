import {Gtk} from "ags/gtk4"
import {shAsync} from "../../../../lib/ExternalCommand"

export default function BluetoothButtonQS(
    {minWidth}: { minWidth: number },
) {
    async function notifyError() {
        shAsync(`notify-send.sh "Bluetooth" "Cannot open Overskride" -i io.github.kaii_lb.Overskride -a Overskride -t 5000`)
    }


    return (
        <button
            css={`
                min-width: ${minWidth}px;
            `}
            onClicked={async () => {
                await shAsync(`overskride`)
                    .catch((_) => notifyError())
            }}
        >
            <box spacing={8}>
                <image
                    iconName={"bluetooth-symbolic"}
                    iconSize={Gtk.IconSize.NORMAL}
                />
                <label label={"Bluetooth"}/>
            </box>
        </button>
    );
}
