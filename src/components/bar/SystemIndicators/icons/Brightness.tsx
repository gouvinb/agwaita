import Brightness from "../../../../services/Brightness"
import {createState} from "ags"
import {Gtk} from "../../../../../../../../../usr/share/ags/js/lib/gtk4"
import {Dimensions} from "../../../../lib/ui/Dimensions"

interface BrightnessIconProps {
    onClicked?: () => void,
    brightness: Brightness
}

export default function BrightnessIcon(
    {
        onClicked,
        brightness,
    }: BrightnessIconProps
) {

    const [icon, setIcon] = createState<string>(resolveIcon(brightness.screen))

    brightness.connect("notify::screen", () => {
        const newIcon = resolveIcon(brightness.screen)
        if (icon.peek() !== newIcon) {
            setIcon(newIcon)
        }
    })


    function resolveIcon(value: number) {
        if (value < 0.20) {
            return "weather-clear-night-symbolic"
        } else if (value < 0.40) {
            return "daytime-sunset-symbolic"
        } else if (value < 0.60) {
            return "daytime-sunrise-symbolic"
        } else {
            return "display-brightness"
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
                sensitive={false}
                hexpand={false}
                onClicked={onClicked}
                iconName={icon}
            />
        )
    )
}
