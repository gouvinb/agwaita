import {Gtk} from "ags/gtk4"
import {Dimensions} from "../../../../lib/ui/Dimensions";
import app from "ags/gtk4/app";

export default function BluetoothButtonQS(
    {minWidth}: { minWidth: number },
) {
    return (
        <button
            css={`
                min-width: ${minWidth}px;
            `}
            onClicked={async () => {
                const bluetoothWindow = app.get_window("bluetoothctl.gui")
                if (!bluetoothWindow) {
                    throw "bluetoothctl.gui window not found"
                }
                bluetoothWindow.show()

            }}
        >
            <box spacing={Dimensions.normalSpacing}>
                <image
                    iconName={"bluetooth-symbolic"}
                    iconSize={Gtk.IconSize.NORMAL}
                />
                <label label={"Bluetooth"}/>
            </box>
        </button>
    );
}
