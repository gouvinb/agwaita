import AstalNotifd from "gi://AstalNotifd"
import DoNotDisturbIcon from "../icons/DoNotDisturb"
import {Dimensions} from "../../../../lib/ui/Dimensions";

interface DotNotDisturbButtonQSProps {
    notifd: AstalNotifd.Notifd,
    minWidth: number
}

export default function DoNotDisturbButtonQS(
    {
        notifd,
        minWidth
    }: DotNotDisturbButtonQSProps,
) {
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
                <DoNotDisturbIcon notifd={notifd}/>
                <label label={"Do not disturb"}/>
            </box>
        </togglebutton>
    );
}
