import {Gtk} from "ags/gtk4"
import PowerProfiles from "gi://AstalPowerProfiles"
import {createBinding, With} from "ags"
import {Dimensions} from "../../../../../lib/ui/Diemensions";

export default function PowerModeRevealerQS(
    {ref}: { ref?: (element: Gtk.Revealer) => void }
) {
    const powerprofiles = PowerProfiles.get_default()

    let powerSaverTB: Gtk.ToggleButton
    let balancedTB: Gtk.ToggleButton
    let performanceTB: Gtk.ToggleButton

    const activeProfile = createBinding(powerprofiles, "activeProfile")

    function updateTB(activeProfile: string) {
        powerSaverTB.active = (activeProfile == "power-saver")
        balancedTB.active = (activeProfile == "balanced")
        performanceTB.active = (activeProfile == "performance")
    }

    powerprofiles.connect("notify::active-profile", ({activeProfile}) => {
        updateTB(activeProfile)
    })

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
                spacing={Dimensions.normalSpacing}
            >
                <box
                    hexpand
                    spacing={Dimensions.bigSpacing}
                    marginBottom={Dimensions.normalSpacing}
                >
                    <image
                        iconName="resources-symbolic"
                        iconSize={Gtk.IconSize.LARGE}
                    />
                    <label
                        css={`
                            font-size: large;
                            font-weight: bold;
                        `}
                        label={"Power profiles"}
                    />
                </box>
                <togglebutton
                    $={(self) => powerSaverTB = self}
                    active={(powerprofiles.activeProfile == "power-saver")}
                    onClicked={() => {
                        powerprofiles.activeProfile = "power-saver"
                    }}
                >
                    <box spacing={Dimensions.normalSpacing}>
                        <image
                            iconName="power-profile-power-saver-symbolic"
                            iconSize={Gtk.IconSize.NORMAL}
                        />
                        <label label={"Power saver"}/>
                        <With value={activeProfile}>
                            {(profile) => (profile === "power-saver") && <image
                                hexpand
                                halign={Gtk.Align.END}
                                iconName={"checkmark-symbolic"}
                                iconSize={Gtk.IconSize.NORMAL}
                            />}
                        </With>
                    </box>
                </togglebutton>
                <togglebutton
                    $={(self) => balancedTB = self}
                    active={(powerprofiles.activeProfile == "balanced")}
                    onClicked={() => {
                        powerprofiles.activeProfile = "balanced"
                    }}
                >
                    <box spacing={Dimensions.normalSpacing}>
                        <image
                            iconName="power-profile-balanced-symbolic"
                            iconSize={Gtk.IconSize.NORMAL}
                        />
                        <label label={"Balanced"}/>
                        <With value={activeProfile}>
                            {(profile) => (profile === "balanced") && <image
                                hexpand
                                halign={Gtk.Align.END}
                                iconName={"checkmark-symbolic"}
                                iconSize={Gtk.IconSize.NORMAL}
                            />}
                        </With>
                    </box>
                </togglebutton>
                <togglebutton
                    $={(self) => performanceTB = self}
                    active={(powerprofiles.activeProfile == "performance")}
                    onClicked={() => {
                        powerprofiles.activeProfile = "performance"
                    }}
                >
                    <box spacing={Dimensions.normalSpacing}>
                        <image
                            iconName="power-profile-performance-symbolic"
                            iconSize={Gtk.IconSize.NORMAL}
                        />
                        <label label={"Performance"}/>
                        <With value={activeProfile}>
                            {(profile) => (profile === "performance") && <image
                                hexpand
                                halign={Gtk.Align.END}
                                iconName={"checkmark-symbolic"}
                                iconSize={Gtk.IconSize.NORMAL}
                            />}
                        </With>
                    </box>
                </togglebutton>
            </box>
        </revealer>
    );
}
