import {createBinding, createEffect, createState, With} from "ags"
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

    const [icon, setIcon] = createState<string>("bluetooth-active-symbolic")
    const [connectedDevices, setConnectedDevices] = createState<number>(0)

    createEffect(() => {
        let newIcon = "bluetooth-active-symbolic"
        if (!isPowered()) {
            newIcon = "bluetooth-hardware-disabled-symbolic"
        } else if (devices().filter((device) => device.connected).length == 0) {
            newIcon = "bluetooth-disabled-symbolic"
        }
        setIcon(newIcon)

        let newCount: number
        if (!isPowered() && !isConnected()) {
            newCount = 0
        } else {
            newCount = Array.from(devices()).filter((device) => device.connected).length
        }
        setConnectedDevices(newCount)
    })

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
                    label={'<span baseline_shift="superscript">' + connectedDevices.peek().toString() + '</span>'}
                />
                }
            </With>
        </box>
    )
}
