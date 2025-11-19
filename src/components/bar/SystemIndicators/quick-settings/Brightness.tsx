import BrightnessIcon from "../icons/Brightness"
import Brightness from "../../../../services/Brightness"
import GLib from "gi://GLib?version=2.0"
import {Astal} from "ags/gtk4"
import {Dimensions} from "../../../../lib/ui/Dimensions";

interface BrightnessQSProps {
    brightness: Brightness
}

export default function BrightnessQS({brightness}: BrightnessQSProps) {
    let slider: Astal.Slider

    let isSliding = false
    let slideTimeout: GLib.Source | null = null

    const setSliding = () => {
        isSliding = true;
        if (slideTimeout) {
            clearTimeout(slideTimeout);
        }
        slideTimeout = setTimeout(() => {
            isSliding = false;
        }, 167);
    };

    brightness.connect("notify::screen", () => {
        if (!isSliding) {
            slider.value = brightness.screen;
        }
    });

    return (
        <box spacing={Dimensions.smallSpacing}>
            <BrightnessIcon
                brightness={brightness}
                onClicked={() => {
                }}
            />
            <slider
                $={(self) => slider = self}
                hexpand
                value={brightness.screen}
                min={0}
                max={1}
                step={0.1}
                onChangeValue={({value}) => {
                    setSliding();
                    brightness.set({screen: value});
                }}
            />
        </box>
    )
}
