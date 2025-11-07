import AstalNotifd from "gi://AstalNotifd"
import DoNotDisturbIcon from "../icons/DoNotDisturb"
import {Dimensions} from "../../../../lib/ui/Dimensions";

export default function DoNotDisturbButtonQS(
    {minWidth}: { minWidth: number },
) {
    const notifd = AstalNotifd.get_default()

    function toggleDnd() {
        notifd.dontDisturb = !notifd.dontDisturb
    }

    return (
        <togglebutton
            css={`
                min-width: ${minWidth}px;
            `}
            active={notifd.dontDisturb}
            onClicked={() => toggleDnd()}
        >
            <box spacing={Dimensions.normalSpacing}>
                <DoNotDisturbIcon/>
                <label label={"Do not disturb"}/>
            </box>
        </togglebutton>
    );
}
