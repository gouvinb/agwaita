import {createBinding, createState} from "ags"
import AstalWp from "gi://AstalWp"
import {Accessor} from "gnim"
import {Gtk} from "ags/gtk4"
import {Dimensions} from "../../../../lib/ui/Dimensions";

export default function AudioIcon(
    {onClicked}: { onClicked?: () => void }
) {
    const {defaultSpeaker: speaker} = AstalWp.get_default()!

    const defaultSpeakerVolume: Accessor<number> = createBinding(speaker, "volume")
    const defaultSpeakerIsMuted: Accessor<boolean> = createBinding(speaker, "mute")

    const [icon, setIcon] = createState<string>(resolveIcon())

    function defaultSpeakerUpdateCallback() {
        return () => {
            const newIcon = resolveIcon()
            if (icon.get() != newIcon) {
                setIcon(newIcon)
            }
        }
    }

    defaultSpeakerVolume.subscribe(defaultSpeakerUpdateCallback())
    defaultSpeakerIsMuted.subscribe(defaultSpeakerUpdateCallback())

    function resolveIcon() {
        const volumePercent = defaultSpeakerVolume.get() * 100
        if (volumePercent == 0 || defaultSpeakerIsMuted.get()) {
            return "audio-volume-muted-symbolic"
        } else if (volumePercent < 34) {
            return "audio-volume-low-symbolic"
        } else if (volumePercent < 67) {
            return "audio-volume-medium-symbolic"
        } else if (volumePercent <= 100) {
            return "audio-volume-high-symbolic"
        } else {
            return "audio-volume-overamplified-symbolic"
        }
    }

    return (
        onClicked === undefined ? (
            <image
                iconName={icon}
                iconSize={Gtk.IconSize.NORMAL}/>
        ) : (
            <button
                css={`
                    margin-left: -${Dimensions.normalSpacing}px;
                `}
                hexpand={false}
                onClicked={onClicked}
                iconName={icon}
            />
        )
    )
}
