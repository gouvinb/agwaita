import BrightnessIcon from "../icons/Brightness"
import Brightness from "../../../../services/Brightness"
import GLib from "gi://GLib?version=2.0"
import {Astal} from "ags/gtk4"
import {Dimensions} from "../../../../lib/ui/Diemensions";

export default function BrightnessQS() {
    const brightnessInstance = Brightness.get_default()

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

    brightnessInstance.connect("notify::screen", () => {
        if (!isSliding) {
            slider.value = brightnessInstance.screen;
        }
    });

    return (
        <box spacing={Dimensions.smallSpacing}>
            <BrightnessIcon onClicked={() => {
            }}/>
            <slider
                $={(self) => slider = self}
                hexpand
                value={brightnessInstance.screen}
                min={0}
                max={1}
                step={0.1}
                onChangeValue={({value}) => {
                    setSliding();
                    brightnessInstance.set({screen: value});
                }}
            />
        </box>
    )
}
