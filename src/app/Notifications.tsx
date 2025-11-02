import {Astal, Gtk} from "ags/gtk4"
import app from "ags/gtk4/app"
import {Accessor, createBinding, createComputed, createState, For, onCleanup} from "ags"
import Notification from "../components/notifications/Notification"
import AstalNotifd from "gi://AstalNotifd"


export const [notifications, setNotifications] = createState(
    new Array<AstalNotifd.Notification>(),
)

export const [notificationsOverlay, setNotificationsOverlay] = createState(
    new Array<AstalNotifd.Notification>(),
)

export default function Notifications() {

    const notifd = AstalNotifd.get_default()

    setNotifications(notifd.get_notifications())
    setNotificationsOverlay(notifd.get_notifications())

    const doNotDisturb: Accessor<boolean> = createBinding(notifd, "dontDisturb")

    const notifiedHandler = notifd.connect("notified", (_, id, replaced) => {
        const newNotifList = (value: AstalNotifd.Notification[]) => {
            if (replaced && notifications.get().some((n) => n.id === id)) {
                return value
                    .map((n) => n.id === id ? notification : n)
                    .filter((n) => n != null)
            } else {
                return [notification, ...value]
                    .filter((n) => n != null)
            }
        };

        const notification = notifd.get_notification(id)

        setNotifications(newNotifList)
        setNotificationsOverlay(newNotifList)
    })

    const resolvedHandler = notifd.connect("resolved", (_, id) => {
        const notificationsResolved = (value: AstalNotifd.Notification[]) => value.filter((n) => n.id !== id);

        setNotifications(notificationsResolved)
        setNotificationsOverlay(notificationsResolved)
    })

    onCleanup(() => {
        notifd.disconnect(notifiedHandler)
        notifd.disconnect(resolvedHandler)
    })

    const visible = createComputed([notificationsOverlay, doNotDisturb], (ns, dnd) => ns.length > 0 && !dnd)

    return (
        <window
            $={(self) => onCleanup(() => self.destroy())}
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
                        padding: 8px;
                    `}
                    orientation={Gtk.Orientation.VERTICAL}
                    spacing={4}
                >
                    <For each={notificationsOverlay}>
                        {(notification) => <Notification
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
                                        timeout_duration)

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
