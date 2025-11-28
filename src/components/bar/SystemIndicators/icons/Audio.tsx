import {createBinding, createEffect, createState} from "ags"
import AstalWp from "gi://AstalWp"
import {Accessor} from "gnim"
import {Gtk} from "ags/gtk4"
import {Dimensions} from "../../../../lib/ui/Dimensions"

interface AudioIconProps {
    onClicked?: () => void,
    wp: AstalWp.Wp
}

export default function AudioIcon({onClicked, wp}: AudioIconProps) {
    const {defaultSpeaker: speaker} = wp

    const defaultSpeakerVolume: Accessor<number> = createBinding(speaker, "volume")
    const defaultSpeakerIsMuted: Accessor<boolean> = createBinding(speaker, "mute")

    const [icon, setIcon] = createState<string>("audio-volume-overamplified-symbolic")

    createEffect(() => {
        const volumePercent = defaultSpeakerVolume() * 100
        let newIcon: string = "audio-volume-overamplified-symbolic"
        if (volumePercent == 0 || defaultSpeakerIsMuted()) {
            newIcon = "audio-volume-muted-symbolic"
        } else if (volumePercent < 34) {
            newIcon = "audio-volume-low-symbolic"
        } else if (volumePercent < 67) {
            newIcon = "audio-volume-medium-symbolic"
        } else if (volumePercent <= 100) {
            newIcon = "audio-volume-high-symbolic"
        }

        if (icon.peek() != newIcon) {
            setIcon(newIcon)
        }
    })


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
