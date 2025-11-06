import {Gtk} from "ags/gtk4"
import {createState} from "ags"
import {interval, Timer} from "ags/time"
import {shAsync} from "../../../../../lib/ExternalCommand";
import {Dimensions} from "../../../../../lib/ui/Diemensions";
import {Log} from "../../../../../lib/Logger";
import {Lifecycle} from "../../../../../lib/Lifecyle";

export default function AccentColorRevealerQS(
    {ref, parentLifecycle = null}: {
        ref?: (element: Gtk.Revealer) => void,
        parentLifecycle?: Lifecycle | null,
    }
) {

    const [accentColor, setAccentColor] = createState<string>("blue");

    let accentColorStateTimer: Timer | null = null

    let blueTB: Gtk.ToggleButton
    let tealTB: Gtk.ToggleButton
    let greenTB: Gtk.ToggleButton
    let yellowTB: Gtk.ToggleButton
    let orangeTB: Gtk.ToggleButton
    let redTB: Gtk.ToggleButton
    let pinkTB: Gtk.ToggleButton
    let purpleTB: Gtk.ToggleButton
    let slateTB: Gtk.ToggleButton

    function updateTB(color: string) {
        blueTB.active = (color == "blue")
        blueTB.cssClasses = resolveButtonClass(blueTB)
        tealTB.active = (color == "teal")
        tealTB.cssClasses = resolveButtonClass(tealTB)
        greenTB.active = (color == "green")
        greenTB.cssClasses = resolveButtonClass(greenTB)
        yellowTB.active = (color == "yellow")
        yellowTB.cssClasses = resolveButtonClass(yellowTB)
        orangeTB.active = (color == "orange")
        orangeTB.cssClasses = resolveButtonClass(orangeTB)
        redTB.active = (color == "red")
        redTB.cssClasses = resolveButtonClass(redTB)
        pinkTB.active = (color == "pink")
        pinkTB.cssClasses = resolveButtonClass(pinkTB)
        purpleTB.active = (color == "purple")
        purpleTB.cssClasses = resolveButtonClass(purpleTB)
        slateTB.active = (color == "slate")
        slateTB.cssClasses = resolveButtonClass(slateTB)
    }

    function setNewAccentColor(color: string) {
        shAsync(`gsettings set org.gnome.desktop.interface accent-color ${color}`)
            .then(() => {
                updateAccentColorState()
            })
            .catch((err) => Log.e("AccentColorRevealerQS", `Cannot set ${color} accent color`, err));
    }

    function updateAccentColorState() {
        shAsync("gsettings get org.gnome.desktop.interface accent-color")
            .then(output => {
                setAccentColor(output.trim().replaceAll("'", ""))
                updateTB(accentColor.get())
            })
            .catch((err) => Log.e("AccentColorRevealerQS", `Cannot get accent color`, err));
    }

    function resolveButtonClass(btn: Gtk.ToggleButton) {
        const classes = []
        if (btn.active) {
            classes.push("active")
        } else {
            classes.push("inactive")
        }
        return classes
    }

    if (parentLifecycle !== null) {
        parentLifecycle.onStart(() => {
            if (accentColorStateTimer == null) {
                accentColorStateTimer = interval(1000, () => updateAccentColorState());
            }
        })
        parentLifecycle.onStop(() => {
            accentColorStateTimer?.cancel()
            accentColorStateTimer = null
        })
    }

    updateAccentColorState();

    return (
        <revealer
            $={(self) => ref?.(self)}
            hexpand
            transitionType={Gtk.RevealerTransitionType.SLIDE_DOWN}
            revealChild={false}
        >
            <box
                css={`
                    padding: ${Dimensions.bigSpacing}px;
                    margin: ${Dimensions.normalSpacing}px 0;
                    border-radius: ${Dimensions.bigSpacing}px;
                `}
                cssClasses={["qs-revealer"]}
                orientation={Gtk.Orientation.VERTICAL}
            >
                <box
                    hexpand
                    spacing={Dimensions.bigSpacing}
                    marginBottom={Dimensions.bigSpacing}
                >
                    <image
                        iconName="preferences-color-symbolic"
                        iconSize={Gtk.IconSize.LARGE}
                    />
                    <label
                        css={`
                            font-size: large;
                            font-weight: bold;
                        `}
                        label={"Accent color"}
                    />
                </box>

                <Gtk.FlowBox
                    cssClasses={["accent-color-flowbox"]}
                    columnSpacing={Dimensions.smallSpacing}
                    rowSpacing={Dimensions.normalSpacing}
                    homogeneous
                    selectionMode={Gtk.SelectionMode.NONE}
                >
                    <togglebutton
                        $={(self) => (blueTB = self)}
                        css={`
                            background-color: var(--accent-blue);
                        `}
                        halign={Gtk.Align.CENTER}
                        onToggled={(btn) => {
                            if (btn.active) setNewAccentColor("blue")
                        }}
                    />
                    <togglebutton
                        $={(self) => (tealTB = self)}
                        css={`
                            background-color: var(--accent-teal);
                        `}
                        halign={Gtk.Align.CENTER}
                        onToggled={(btn) => {
                            if (btn.active) setNewAccentColor("teal")
                        }}
                    />
                    <togglebutton
                        $={(self) => (greenTB = self)}
                        css={`
                            background-color: var(--accent-green);
                        `}
                        halign={Gtk.Align.CENTER}
                        onToggled={(btn) => {
                            if (btn.active) setNewAccentColor("green")
                        }}
                    />
                    <togglebutton
                        $={(self) => (yellowTB = self)}
                        css={`
                            background-color: var(--accent-yellow);
                        `}
                        halign={Gtk.Align.CENTER}
                        onToggled={(btn) => {
                            if (btn.active) setNewAccentColor("yellow")
                        }}
                    />
                    <togglebutton
                        $={(self) => (orangeTB = self)}
                        css={`
                            background-color: var(--accent-orange);
                        `}
                        halign={Gtk.Align.CENTER}
                        onToggled={(btn) => {
                            if (btn.active) setNewAccentColor("orange")
                        }}
                    />
                    <togglebutton
                        $={(self) => (redTB = self)}
                        css={`
                            background-color: var(--accent-red);
                        `}
                        halign={Gtk.Align.CENTER}
                        onToggled={(btn) => {
                            if (btn.active) setNewAccentColor("red")
                        }}
                    />
                    <togglebutton
                        $={(self) => (pinkTB = self)}
                        css={`
                            background-color: var(--accent-pink);
                        `}
                        halign={Gtk.Align.CENTER}
                        onToggled={(btn) => {
                            if (btn.active) setNewAccentColor("pink")
                        }}
                    />
                    <togglebutton
                        $={(self) => (purpleTB = self)}
                        css={`
                            background-color: var(--accent-purple);
                        `}
                        halign={Gtk.Align.CENTER}
                        onToggled={(btn) => {
                            if (btn.active) setNewAccentColor("purple")
                        }}
                    />
                    <togglebutton
                        $={(self) => (slateTB = self)}
                        css={`
                            background-color: var(--accent-slate);
                        `}
                        halign={Gtk.Align.CENTER}
                        onToggled={(btn) => {
                            if (btn.active) setNewAccentColor("slate")
                        }}
                    />
                </Gtk.FlowBox>
            </box>
        </revealer>
    );
}
