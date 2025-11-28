import {createBinding, createEffect, createState, With} from "ags"
import AstalBattery from "gi://AstalBattery"
import {Accessor} from "gnim"
import {Gtk} from "ags/gtk4"
import {Dimensions} from "../../../../lib/ui/Dimensions"

interface BatteryIconProps {
    battery: AstalBattery.Device
}

export default function BatteryIcon({battery}: BatteryIconProps) {
    const percentage: Accessor<number> = createBinding(battery, "percentage")
    const isCharging: Accessor<boolean> = createBinding(battery, "charging")
    const isPresent: Accessor<boolean> = createBinding(battery, "isPresent")

    const [icon, setIcon] = createState<string>("battery-missing-symbolic")

    createEffect(() => {
        let newIcon: string

        if (!isPresent()) {
            newIcon = "battery-missing-symbolic"
        } else {
            const percent = Math.round(percentage() * 100)
            const charging = isCharging()

            if (charging) {
                newIcon = "battery-level-0-charging-symbolic";
                if (percent >= 10) newIcon = "battery-level-10-charging-symbolic"
                if (percent >= 20) newIcon = "battery-level-20-charging-symbolic"
                if (percent >= 30) newIcon = "battery-level-30-charging-symbolic"
                if (percent >= 40) newIcon = "battery-level-40-charging-symbolic"
                if (percent >= 50) newIcon = "battery-level-50-charging-symbolic"
                if (percent >= 60) newIcon = "battery-level-60-charging-symbolic"
                if (percent >= 70) newIcon = "battery-level-70-charging-symbolic"
                if (percent >= 80) newIcon = "battery-level-80-charging-symbolic"
                if (percent >= 90) newIcon = "battery-level-90-charging-symbolic"
                if (percent >= 100) newIcon = "battery-full-charging-symbolic"
            } else {
                newIcon = "battery-level-0-symbolic";
                if (percent >= 10) newIcon = "battery-level-10-symbolic"
                if (percent >= 20) newIcon = "battery-level-20-symbolic"
                if (percent >= 30) newIcon = "battery-level-30-symbolic"
                if (percent >= 40) newIcon = "battery-level-40-symbolic"
                if (percent >= 50) newIcon = "battery-level-50-symbolic"
                if (percent >= 60) newIcon = "battery-level-60-symbolic"
                if (percent >= 70) newIcon = "battery-level-70-symbolic"
                if (percent >= 80) newIcon = "battery-level-80-symbolic"
                if (percent >= 90) newIcon = "battery-level-90-symbolic"
                if (percent >= 100) newIcon = "battery-level-100-symbolic"
            }
        }

        if (icon.peek() !== newIcon) {
            setIcon(newIcon);
        }
    })

    return (
        <box spacing={Dimensions.smallSpacing}>
            <image
                iconName={icon}
                iconSize={Gtk.IconSize.NORMAL}
            />
            <With value={isPresent}>
                {(isPresent) => isPresent && (
                    <label label={`${Math.round(percentage.peek() * 100)}%`}/>
                ) || (
                    <label label="!"/>
                )}
            </With>
        </box>
    );
}
