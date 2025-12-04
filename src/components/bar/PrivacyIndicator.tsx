import {createBinding, createEffect, createState, With} from "ags"
import {Gtk} from "ags/gtk4"
import AstalWp from "gi://AstalWp"
import Geoclue from "gi://Geoclue?version=2.0"
import {Log} from "../../lib/Logger";

interface PrivacyUsage {
    camera: Set<string>
    microphone: Set<string>
    location: Set<string>
    screencast: Set<string>
}

interface PrivacyIndicatorProps {
    wp: AstalWp.Wp
}

export default function PrivacyIndicator({wp}: PrivacyIndicatorProps) {
    const [usage, setUsage] = createState<PrivacyUsage>({
        camera: new Set(),
        microphone: new Set(),
        location: new Set(),
        screencast: new Set()
    })
    const [visible, setVisible] = createState<boolean>(false)

    const audio = wp.audio
    const video = wp.video

    const audioRecorders = createBinding(audio, "recorders")
    const videoRecorders = createBinding(video, "recorders")

    const [geoclueInUse, setGeoclueInUse] = createState<boolean>(false)

    createEffect(() => {
        Log.d("PrivacyIndicator", "Creating GeoClue Manager proxy")
        Geoclue.ManagerProxy.new_for_bus(
            1, // Gio.BusType.SYSTEM
            0, // Gio.DBusProxyFlags.NONE
            "org.freedesktop.GeoClue2",
            "/org/freedesktop/GeoClue2/Manager",
            null,
            (_, res) => {
                try {
                    const manager = Geoclue.ManagerProxy.new_for_bus_finish(res)
                    Log.d("PrivacyIndicator", `GeoClue Manager created, InUse: ${manager.in_use}`)

                    manager.connect("notify::in-use", () => {
                        Log.d("PrivacyIndicator", `GeoClue InUse changed: ${manager.in_use}`)
                        setGeoclueInUse(manager.in_use)
                    })

                    setGeoclueInUse(manager.in_use)
                } catch (e) {
                    Log.e("PrivacyIndicator", `Failed to create GeoClue manager: ${e}`)
                }
            }
        )
    })


    // Audio input
    createEffect(() => {
        const recorders = audioRecorders()
        const newMicApps = new Set<string>()

        recorders.forEach((recorder, index) => {
            let appName = `∙ ${recorder.name || `Recorder ${index}`}`
            if (recorder.description && recorder.name != recorder.description) {
                appName += ` <i>(${recorder.description})</i>`
            }

            if (!appName.toLowerCase().includes("pipewire") &&
                !appName.toLowerCase().includes("wireplumber") &&
                !appName.toLowerCase().includes("monitor") &&
                !appName.toLowerCase().includes("built-in audio")) {
                newMicApps.add(appName)
            }
        })

        setUsage(prev => ({
            ...prev,
            microphone: newMicApps
        }))
    })

    // Camera
    createEffect(() => {
        const recorders = videoRecorders()
        const newCameraApps = new Set<string>()

        recorders.forEach((recorder, index) => {
            let appName = `∙ ${recorder.name || `Recorder ${index}`}`
            if (recorder.description && recorder.name != recorder.description) {
                appName += ` <i>(${recorder.description})</i>`
            }
            Log.d("PrivacyIndicator", `Camera app: ${appName}`)
            if (!appName.toLowerCase().includes("pipewire") &&
                !appName.toLowerCase().includes("wireplumber") &&
                !appName.toLowerCase().includes("webrtc-consume-stream")) {
                newCameraApps.add(appName)
            }
        })

        setUsage(prev => ({
            ...prev,
            camera: newCameraApps
        }))
    })

    // Screencast
    createEffect(() => {
        const recorders = videoRecorders()
        const newScreencastApps = new Set<string>()

        recorders.forEach((recorder, index) => {
            let appName = `∙ ${recorder.name || `Recorder ${index}`}`
            if (recorder.description && recorder.name != recorder.description) {
                appName += ` <i>(${recorder.description})</i>`
            }

            if (appName.toLowerCase().includes("webrtc-consume-stream") ||
                appName.toLowerCase().includes("screen") ||
                appName.toLowerCase().includes("desktop")) {
                newScreencastApps.add(appName)
            }
        })

        setUsage(prev => ({
            ...prev,
            screencast: newScreencastApps
        }))
    })

    // Location (GeoClue)
    createEffect(() => {
        const inUse = geoclueInUse()
        const newLocationApps = new Set<string>()

        if (inUse) {
            newLocationApps.add(`∙ Location services active <i>(GeoClue)</i>`)
        }

        setUsage(prev => ({
            ...prev,
            location: newLocationApps
        }))
    })

    createEffect(() => {
        const u = usage()
        setVisible(
            u.camera.size > 0 ||
            u.microphone.size > 0 ||
            u.location.size > 0 ||
            u.screencast.size > 0
        )
    })

    const tooltipText = usage.as(u => {
        const lines: string[] = []

        if (u.microphone.size > 0) {
            lines.push(`<b>Microphone:</b>\n${Array.from(u.microphone).join('\n')}`)
        }
        if (u.camera.size > 0) {
            lines.push(`<b>Camera:</b>\n${Array.from(u.camera).join('\n')}`)
        }
        if (u.location.size > 0) {
            lines.push(`<b>Location:</b>\n${Array.from(u.location).join('\n')}`)
        }
        if (u.screencast.size > 0) {
            lines.push(`<b>Screen sharing:</b>\n${Array.from(u.screencast).join('\n')}`)
        }

        return lines.join('\n\n')
    })

    return (
        <With value={visible}>
            {(isVisible) => (
                isVisible && (
                    <button
                        sensitive={false}
                        tooltipMarkup={tooltipText}
                    >
                        <box
                            spacing={8}
                        >
                            <image
                                css={`
                                    color: var(--accent-color);
                                    opacity: 100%;
                                `}
                                iconName="camera-web-symbolic"
                                iconSize={Gtk.IconSize.NORMAL}
                                visible={usage.as(u => u.camera.size > 0)}
                            />
                            <image
                                css={`
                                    color: var(--accent-color);
                                    opacity: 100%;
                                `}
                                iconName="audio-input-microphone-symbolic"
                                iconSize={Gtk.IconSize.NORMAL}
                                visible={usage.as(u => u.microphone.size > 0)}
                            />
                            <image
                                css={`
                                    color: var(--accent-color);
                                    opacity: 100%;
                                `}
                                iconName="location-services-active-symbolic"
                                iconSize={Gtk.IconSize.NORMAL}
                                visible={usage.as(u => u.location.size > 0)}
                            />
                            <image
                                css={`
                                    color: var(--accent-color);
                                    opacity: 100%;
                                `}
                                iconName="screen-shared-symbolic"
                                iconSize={Gtk.IconSize.NORMAL}
                                visible={usage.as(u => u.screencast.size > 0)}
                            />
                        </box>
                    </button>
                )
            )}
        </With>
    )
}
