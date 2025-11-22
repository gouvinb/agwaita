import {createBinding, createState, With} from "ags"
import AstalBluetooth from "gi://AstalBluetooth"
import {Accessor} from "gnim"
import {Gtk} from "ags/gtk4"

interface BluetoothProps {
    bluetooth: AstalBluetooth.Bluetooth
}

export default function BluetoothIcon({bluetooth}: BluetoothProps) {
    const devices: Accessor<AstalBluetooth.Device[]> = createBinding(bluetooth, "devices")
    const isPowered = createBinding(bluetooth, "isPowered") as Accessor<boolean>
    const isConnected: Accessor<boolean> = createBinding(bluetooth, "isConnected") as Accessor<boolean>

    const [icon, setIcon] = createState<string>(resolveStatusIcon())
    const [connectedDevices, setConnectedDevices] = createState<number>(resolveConnectedDevicesCount())

    function bluetoothUpdateCallback() {
        return () => {
            const newIcon = resolveStatusIcon()
            setIcon(newIcon)

            const newCount = resolveConnectedDevicesCount()
            setConnectedDevices(newCount)
        }
    }

    devices.subscribe(bluetoothUpdateCallback())
    isPowered.subscribe(bluetoothUpdateCallback())
    isConnected.subscribe(bluetoothUpdateCallback())

    function resolveStatusIcon() {
        if (!isPowered.get()) {
            return "bluetooth-hardware-disabled-symbolic"
        } else if (Array.from(devices.get()).filter((device) => device.connected).length == 0) {
            return "bluetooth-disabled-symbolic"
        } else {
            return "bluetooth-active-symbolic"
        }
    }

    function resolveConnectedDevicesCount() {
        if (!isPowered.get() && !isConnected.get()) {
            return 0
        }
        return Array.from(devices.get()).filter((device) => device.connected).length
    }

    return (
        <box>
            <image
                iconName={icon}
                iconSize={Gtk.IconSize.NORMAL}
            />
            <With value={connectedDevices}>
                {(devicesCount) => devicesCount > 0 && <label
                    css={`
                        font-size: xx-small;
                    `}
                    use_markup
                    label={'<span baseline_shift="superscript">' + connectedDevices.get().toString() + '</span>'}
                />
                }
            </With>
        </box>
    )
}
