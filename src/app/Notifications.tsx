import {Astal, Gtk} from "ags/gtk4"
import app from "ags/gtk4/app"
import {Accessor, createBinding, createComputed, createState, For, onCleanup} from "ags"
import Notification from "../components/notifications/Notification"
import AstalNotifd from "gi://AstalNotifd"
import {Dimensions} from "../lib/ui/Dimensions"


export function Notifications(notifd: AstalNotifd.Notifd) {
    const [notificationsOverlay, setNotificationsOverlay] = createState(
        new Array<AstalNotifd.Notification>(),
    )

    setNotificationsOverlay(notifd.get_notifications())

    const doNotDisturb: Accessor<boolean> = createBinding(notifd, "dontDisturb")

    const notifiedHandler = notifd.connect("notified", (_, id, replaced) => {
        const notifications = notifd.get_notifications()
        const newNotifList = (value: AstalNotifd.Notification[]) => {
            if (replaced && notifications.some((n) => n.id === id)) {
                return value
                    .map((n) => n.id === id ? notification : n)
                    .filter((n) => n != null)
            } else {
                return [notification, ...value]
                    .filter((n) => n != null)
            }
        };

        const notification = notifd.get_notification(id)

        setNotificationsOverlay(newNotifList)
    })

    const resolvedHandler = notifd.connect("resolved", (_, id) => {
        const notificationsResolved = (value: AstalNotifd.Notification[]) => value.filter((n) => n.id !== id);

        setNotificationsOverlay(notificationsResolved)
    })

    const visible = createComputed([notificationsOverlay, doNotDisturb], (ns, dnd) => ns.length > 0 && !dnd)

    let win: Astal.Window

    onCleanup(() => {
        notifd.disconnect(notifiedHandler)
        notifd.disconnect(resolvedHandler)
        win.destroy()
    })
    return (
        <window
            $={(self) => win = self}
            cssClasses={["ags-notifications"]}
            visible={visible}
            name="notifications"
            namespace="ags-notifications"
            layer={Astal.Layer.OVERLAY}
            anchor={Astal.WindowAnchor.TOP | Astal.WindowAnchor.RIGHT | Astal.WindowAnchor.BOTTOM}
            application={app}
        >
            <scrolledwindow
                propagateNaturalWidth
                propagateNaturalHeight
            >
                <box
                    css={`
                        padding: ${Dimensions.normalSpacing}px;
                    `}
                    orientation={Gtk.Orientation.VERTICAL}
                    spacing={Dimensions.smallSpacing}
                >
                    <For each={notificationsOverlay.as((n) => n.slice(0, 15))}>
                        {(notification: AstalNotifd.Notification) => <Notification
                            isOverlay
                            init={
                                (n) => {
                                    let timeout_duration = n.expire_timeout
                                    if (timeout_duration <= 0) {
                                        timeout_duration = 5_000
                                    }
                                    const timeoutId = setTimeout(
                                        () => {
                                            setNotificationsOverlay((value) => {
                                                return value.filter((notif) => notif.id !== n.id)
                                            })
                                        },
                                        timeout_duration
                                    )

                                    return () => {
                                        if (timeoutId) {
                                            clearTimeout(timeoutId)
                                        }
                                    }
                                }
                            }
                            notification={notification}
                        />}
                    </For>
                </box>
            </scrolledwindow>
        </window>
    )
}
