import AudioIcon from "../icons/Audio"
import AstalWp from "gi://AstalWp"
import GLib from "gi://GLib?version=2.0"
import {Astal} from "ags/gtk4"
import {shAsync} from "../../../../lib/ExternalCommand"
import {Dimensions} from "../../../../lib/ui/Dimensions";

interface AudioQsProps {
    wp: AstalWp.Wp
}

export default function AudioQS({wp}: AudioQsProps) {
    const {defaultSpeaker: speaker} = AstalWp.get_default()!

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

    speaker.connect("notify::volume", () => {
        if (!isSliding) {
            slider.value = speaker.volume;
        }
    });

    return (
        <box spacing={Dimensions.smallSpacing}>
            <AudioIcon
                wp={wp}
                onClicked={() => shAsync("pavucontrol")}
            />
            <slider
                $={(self) => (slider = self)}
                min={0}
                max={1.5}
                step={0.01}
                hexpand
                value={speaker.volume}
                onChangeValue={({value}) => {
                    setSliding();
                    speaker.set_volume(value);
                }}
            />
        </box>
    );
}
