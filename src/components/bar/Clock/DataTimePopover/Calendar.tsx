import {Gtk} from "ags/gtk4";
import {Accessor} from "ags";
import {createPoll} from "ags/time"
import GLib from "gi://GLib"
import "../../../../lib/extension/String"

export function Calendar(
    {ref, markCalendar, updateDaySelected}: {
        ref: (instance: Gtk.Calendar) => void,
        markCalendar: () => void,
        updateDaySelected: () => void,
    },
) {
    const rawDateTime: Accessor<GLib.DateTime> = createPoll(
        GLib.DateTime.new_now_local(),
        1000,
        () => GLib.DateTime.new_now_local(),
    )

    const dayWeek = rawDateTime.as((data) => data.format("%A")!.capitalize())
    const date = rawDateTime.as((data) => data.format("%e %B %Y")!.capitalize())

    return (
        <box orientation={Gtk.Orientation.VERTICAL} spacing={4}>
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
                    self.connect("notify::month", () => {
                        markCalendar()
                        updateDaySelected()
                    })
                    self.connect("day-selected", () => {
                        updateDaySelected()
                    })
                    ref(self)
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
