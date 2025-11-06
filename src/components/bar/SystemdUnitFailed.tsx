import {createState, With} from "ags"
import {shAsync} from "../../lib/ExternalCommand"
import {interval} from "ags/time"
import {Dimensions} from "../../lib/ui/Diemensions";
import {Gtk} from "ags/gtk4";

export default function SystemdUnitFailed() {
    const [_, setFailedUnit] = createState<string[]>([]);
    const [failedUnitCount, setFailedUnitCount] = createState<number>(0);
    const [failedUnitVisible, setFailedUnitVisible] = createState<boolean>(true);
    const [failedUnitSuccess, setFailedUnitSuccess] = createState<boolean>(true);
    const [icon, setIcon] = createState<string>("");

    async function updateFailedUnit() {
        try {
            const output = await shAsync("systemctl --user --failed --no-legend");

            const units = output
                .trim()
                .split('\n')
                .filter(line => {
                    const trimmed = line.trim();
                    return trimmed.length > 0 && (trimmed.startsWith('●') || trimmed.includes('failed'));
                });

            setFailedUnit(units);

            const count = units.length;
            setFailedUnitCount(count);

            const visible = units.length != 0;
            setFailedUnitVisible(visible);

            const newIcon = resolveIcon(count);
            setIcon(newIcon);

            setFailedUnitSuccess(true);
        } catch (error) {
            console.error("Erreur lors de la récupération des unités systemd:", error);
            setFailedUnit([]);
            setFailedUnitCount(-1);
            setIcon(resolveIcon(-1));
            setFailedUnitSuccess(false);
        }
    }

    function resolveIcon(count: number) {
        if (count > 0) return "software-update-urgent-symbolic";
        if (count < 0) return "computer-fail-symbolic";
        return "";
    }

    updateFailedUnit();

    interval(5000, () => {
        updateFailedUnit();
    });

    return (
        <With value={failedUnitVisible}>
            {(failedUnit) => (
                failedUnit && <box
                    css={`
                        padding-right: ${Dimensions.smallSpacing}px;
                    `}>
                    <image
                        iconName={icon}
                        iconSize={Gtk.IconSize.NORMAL}
                    />
                    <With value={failedUnitSuccess}>
                        {(failedUnitSuccess) => (
                            failedUnitSuccess && <label
                                use_markup
                                label={failedUnitCount.as(count =>
                                    ` <span baseline_shift="superscript" font_scale="superscript">${count}</span>`
                                )}
                            />
                        )}
                    </With>
                </box>
            )}
        </With>
    );
}

