import {Gtk} from "ags/gtk4";
import {interval, Timer} from "ags/time"
import GLib from "gi://GLib"
import "../../../../lib/extension/String"
import {Dimensions} from "../../../../lib/ui/Diemensions";
import {Lifecycle} from "../../../../lib/Lifecyle";
import {createState} from "ags";

export function Calendar(
    {ref, parentLifecycle, markCalendar, updateDaySelected}: {
        ref: (instance: Gtk.Calendar) => void,
        parentLifecycle: Lifecycle,
        markCalendar: (calendar: Gtk.Calendar) => void,
        updateDaySelected: (calendar: Gtk.Calendar) => void,
    },
) {
    const [rawDateTime, setRawDateTime] = createState<GLib.DateTime>(GLib.DateTime.new_now_local())

    let rawDateTimeTimer: Timer | null = null

    const dayWeek = rawDateTime.as((data) => data.format("%A")!.capitalize())
    const date = rawDateTime.as((data) => data.format("%e %B %Y")!.capitalize())

    let calendar: Gtk.Calendar;

    parentLifecycle.onStart(() => {
        rawDateTimeTimer = interval(
            1000,
            () => setRawDateTime(GLib.DateTime.new_now_local()),
        )
        calendar.show()
    })
    parentLifecycle.onStop(() => {
        rawDateTimeTimer?.cancel()
        rawDateTimeTimer = null
        calendar.hide()
        calendar.set_date(rawDateTime.get())
        calendar.select_day(rawDateTime.get())
    })

    return (
        <box orientation={Gtk.Orientation.VERTICAL} spacing={Dimensions.smallSpacing}>
            <label
                css={`
                    font-weight: bold;
                `}
                label={dayWeek}
            />
            <label
                css={`
                    font-weight: bold;
                    font-size: large;
                `}
                label={date}
            />
            <Gtk.Calendar
                $={(self) => {
                    calendar = self
                    ref(self)
                    self.connect("notify::month", () => {
                        markCalendar(self)
                        updateDaySelected(self)
                    })
                    self.connect("day-selected", () => {
                        updateDaySelected(self)
                    })
                }}
                css={`
                    background: none;
                    border: none;
                    padding: 0;
                `}
            />
        </box>
    )
}
