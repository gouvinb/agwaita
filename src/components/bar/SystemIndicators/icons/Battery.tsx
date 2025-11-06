import {createBinding, createState, With} from "ags"
import AstalBattery from "gi://AstalBattery"
import {Accessor} from "gnim"
import {Gtk} from "ags/gtk4";
import {Dimensions} from "../../../../lib/ui/Diemensions";

export default function BatteryIcon() {
    const battery = AstalBattery.get_default()

    const percentage: Accessor<number> = createBinding(battery, "percentage")
    const isCharging: Accessor<boolean> = createBinding(battery, "charging")
    const isPresent: Accessor<boolean> = createBinding(battery, "isPresent")

    const [icon, setIcon] = createState<string>("battery-missing-symbolic")

    function resolveIcon(): string {
        if (!isPresent.get()) {
            return "battery-missing-symbolic"
        }

        const percent = Math.round(percentage.get() * 100)
        const charging = isCharging.get();

        if (charging) {
            if (percent >= 100) return "battery-full-charging-symbolic";
            if (percent >= 90) return "battery-level-90-charging-symbolic";
            if (percent >= 80) return "battery-level-80-charging-symbolic";
            if (percent >= 70) return "battery-level-70-charging-symbolic";
            if (percent >= 60) return "battery-level-60-charging-symbolic";
            if (percent >= 50) return "battery-level-50-charging-symbolic";
            if (percent >= 40) return "battery-level-40-charging-symbolic";
            if (percent >= 30) return "battery-level-30-charging-symbolic";
            if (percent >= 20) return "battery-level-20-charging-symbolic";
            if (percent >= 10) return "battery-level-10-charging-symbolic";
            return "battery-level-0-charging-symbolic";
        } else {
            if (percent >= 100) return "battery-level-100-symbolic";
            if (percent >= 90) return "battery-level-90-symbolic";
            if (percent >= 80) return "battery-level-80-symbolic";
            if (percent >= 70) return "battery-level-70-symbolic";
            if (percent >= 60) return "battery-level-60-symbolic";
            if (percent >= 50) return "battery-level-50-symbolic";
            if (percent >= 40) return "battery-level-40-symbolic";
            if (percent >= 30) return "battery-level-30-symbolic";
            if (percent >= 20) return "battery-level-20-symbolic";
            if (percent >= 10) return "battery-level-10-symbolic";
            return "battery-level-0-symbolic";
        }
    }

    function updateIcon() {
        const newIcon = resolveIcon();
        if (icon.get() !== newIcon) {
            setIcon(newIcon);
        }
    }

    updateIcon();

    percentage.subscribe(() => updateIcon());
    isCharging.subscribe(() => updateIcon());
    isPresent.subscribe(() => updateIcon());

    return (
        <box spacing={Dimensions.smallSpacing}>
            <image
                iconName={icon}
                iconSize={Gtk.IconSize.NORMAL}
            />
            <With value={isPresent}>
                {(isPresent) => isPresent && (
                    <label label={`${Math.round(percentage.get() * 100)}%`}/>
                ) || (
                    <label label="!"/>
                )}
            </With>
        </box>
    );
}
