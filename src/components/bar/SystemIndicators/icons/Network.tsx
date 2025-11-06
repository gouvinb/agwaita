import {createState} from "ags"
import {interval} from "ags/time"
import GLib from "gi://GLib"
import {Gtk} from "ags/gtk4";

type NetworkState = {
    type: "wired" | "wifi" | "none"
    connected: boolean
    wifiStrength?: number
    interface?: string
}

export default function NetworkIcon() {
    const [icon, setIcon] = createState<string>("network-wireless-disabled-symbolic")

    function getNetworkState(): NetworkState {
        try {
            const [, stdout] = GLib.spawn_command_line_sync("ip -br link show up")
            let output: string = "";
            if (stdout) output = new TextDecoder().decode(stdout);

            const lines = output.trim().split("\n");

            const wiredInterfaces = lines.filter(line =>
                !line.startsWith("lo") &&
                (line.includes("eth") || line.includes("enp") || line.includes("eno"))
            );

            const wifiInterfaces = lines.filter(line =>
                !line.startsWith("lo") &&
                (line.includes("wlan") || line.includes("wlp"))
            );

            const [, routeOut] = GLib.spawn_command_line_sync("ip route show default");
            let routeOutput: string = "";
            if (routeOut) routeOutput = new TextDecoder().decode(routeOut);

            const hasDefaultRoute = routeOutput.trim().length > 0;

            const wifiInterfaceDefault = wifiInterfaces[0]
            const wiredInterfaceDefault = wiredInterfaces[0]

            if (!hasDefaultRoute) {
                if (wifiInterfaceDefault) {
                    return {type: "wifi", connected: false, interface: wifiInterfaceDefault.split(/\s+/)[0]};
                }
                if (wiredInterfaceDefault) {
                    return {type: "wired", connected: false, interface: wiredInterfaceDefault.split(/\s+/)[0]};
                }
                return {type: "none", connected: false};
            }

            if (wiredInterfaceDefault != null && (routeOutput.includes("eth") || routeOutput.includes("enp") || routeOutput.includes("eno"))) {
                return {type: "wired", connected: true, interface: wiredInterfaceDefault.split(/\s+/)[0]};
            }

            if (wifiInterfaceDefault) {
                const wifiInterface = wifiInterfaceDefault.split(/\s+/)[0];

                try {
                    const [, iwOut] = GLib.spawn_command_line_sync(`iw dev ${wifiInterface} link`);
                    let iwOutput: string = "";
                    if (iwOut) iwOutput = new TextDecoder().decode(iwOut);

                    if (iwOutput.includes("Not connected")) {
                        return {type: "wifi", connected: false, interface: wifiInterface};
                    }

                    const signalMatch = iwOutput.match(/signal:\s*(-?\d+)\s*dBm/);
                    if (signalMatch && signalMatch[1]) {
                        const signalDbm = parseInt(signalMatch[1]);
                        // -30 dBm = excellent (100%), -90 dBm = very low (0%)
                        const strength = Math.min(100, Math.max(0, ((signalDbm + 90) / 60) * 100));
                        return {type: "wifi", connected: true, wifiStrength: strength, interface: wifiInterface};
                    }

                    return {type: "wifi", connected: true, wifiStrength: 50, interface: wifiInterface};
                } catch {
                    return {type: "wifi", connected: true, wifiStrength: 50, interface: wifiInterface};
                }
            }

            return {type: "none", connected: false};
        } catch (error) {
            printerr(error);
            return {type: "none", connected: false};
        }
    }

    function resolveStatusIcon(state: NetworkState): string {
        if (state.type === "wired") {
            return state.connected
                ? "network-wired-symbolic"
                : "network-wired-disconnected-symbolic";
        }

        if (state.type === "wifi") {
            if (!state.connected) {
                return "network-wireless-offline-symbolic";
            }

            const strength = state.wifiStrength ?? 0;
            if (strength >= 80) return "network-wireless-signal-excellent-symbolic";
            if (strength >= 60) return "network-wireless-signal-good-symbolic";
            if (strength >= 40) return "network-wireless-signal-ok-symbolic";
            if (strength >= 20) return "network-wireless-signal-weak-symbolic";
            return "network-wireless-signal-none-symbolic";
        }

        return "network-wireless-disabled-symbolic";
    }

    function updateNetworkStatus() {
        const state = getNetworkState();
        const newIcon = resolveStatusIcon(state);
        setIcon(newIcon);
    }

    updateNetworkStatus();

    interval(1000, () => {
        updateNetworkStatus();
    });

    return (
        <image
            iconName={icon}
            iconSize={Gtk.IconSize.NORMAL}
        />
    );
}
