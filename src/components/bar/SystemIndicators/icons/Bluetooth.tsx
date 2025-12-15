import {createBinding, createEffect, createState, onCleanup, With} from "ags"
import AstalBluetooth from "gi://AstalBluetooth"
import {Gtk} from "ags/gtk4"

interface BluetoothProps {
    bluetooth: AstalBluetooth.Bluetooth
}

export default function BluetoothIcon({bluetooth}: BluetoothProps) {
    const currentAdapter = createBinding(bluetooth, "adapter")

    const [powerState, setPowerState] = createState(currentAdapter.peek()?.powered ?? false)
    const [devices, setDevices] = createState<AstalBluetooth.Device[]>([]);


    const [icon, setIcon] = createState<string>("bluetooth-active-symbolic")
    const [connectedDevices, setConnectedDevices] = createState<number>(0)

    const signalHandlers: number[] = []
    const deviceSignalHandlers = new Map<string, number[]>()

    function setupAdapterSignals(adapter: AstalBluetooth.Adapter | null) {
        if (!adapter) return

        signalHandlers.push(
            adapter.connect("notify::powered", () => {
                setPowerState(adapter.powered)
            })
        )
    }

    function setupDeviceSignals(device: AstalBluetooth.Device) {
        const deviceId = device.address
        if (deviceSignalHandlers.has(deviceId)) return

        const handlers: number[] = []

        const properties = ['connected', 'paired', 'trusted', 'battery-percentage', 'alias', 'rssi']
        properties.forEach(prop => {
            handlers.push(
                device.connect(`notify::${prop}`, () => updateDevices())
            )
        })

        deviceSignalHandlers.set(deviceId, handlers)
    }

    function cleanupDeviceSignals(device: AstalBluetooth.Device) {
        const deviceId = device.address
        const handlers = deviceSignalHandlers.get(deviceId)
        if (handlers) {
            handlers.forEach(id => device.disconnect(id))
            deviceSignalHandlers.delete(deviceId)
        }
    }

    function updateDevices() {
        const newDevices = bluetooth.get_devices()

        newDevices.forEach(device => setupDeviceSignals(device))

        setDevices(newDevices)
    }

    createEffect(() => {
        const adapter = currentAdapter()
        setupAdapterSignals(adapter)

        signalHandlers.push(
            bluetooth.connect("device-added", () => updateDevices())
        )
        signalHandlers.push(
            bluetooth.connect("device-removed", (_, device) => {
                cleanupDeviceSignals(device)
                updateDevices()
            })
        )

        updateDevices()
    })

    onCleanup(() => {
        const adapter = currentAdapter.peek()

        if (adapter) {
            signalHandlers.forEach(id => {
                try {
                    adapter.disconnect(id)
                } catch {
                    bluetooth.disconnect(id)
                }
            })
        }
        signalHandlers.length = 0

        deviceSignalHandlers.forEach((handlers, deviceId) => {
            const device = bluetooth.get_devices().find(d => d.address === deviceId)
            if (device) {
                handlers.forEach(id => device.disconnect(id))
            }
        })
        deviceSignalHandlers.clear()
    })

    createEffect(() => {
        let newIcon = "bluetooth-active-symbolic"
        if (!powerState()) {
            newIcon = "bluetooth-hardware-disabled-symbolic"
        } else if (bluetooth.devices.filter((device) => device.connected).length == 0) {
            newIcon = "bluetooth-disabled-symbolic"
        }
        setIcon(newIcon)

        let newCount: number
        if (!powerState()) {
            newCount = 0
        } else {
            newCount = devices().filter((device) => device.connected).length
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
