import Brightness from "../../../../services/Brightness"
import {createState} from "ags"

export default function BrightnessIcon() {
    const brightnessInstance = Brightness.get_default()

    const [icon, setIcon] = createState<string>(resolveIcon(brightnessInstance.screen))

    brightnessInstance.connect("notify::screen", () => {
        const newIcon = resolveIcon(brightnessInstance.screen)
        if (icon.get() !== newIcon) {
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
        <image
            iconName={icon}
            pixelSize={16}
        />
    )
}
