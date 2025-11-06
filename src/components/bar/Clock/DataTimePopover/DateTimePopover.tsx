import {Accessor, createState, onCleanup} from "ags"
import {Gtk} from "ags/gtk4"
import {createPoll} from "ags/time"
import GLib from "gi://GLib"
import Agenda, {CalendarEvent} from "../../../../services/Agenda"
import {Calendar} from "./Calendar";
import {EventList} from "./EventList";
import {DateTimeExt} from "../../../../lib/extension/GLibDateTime";
import {Dimensions} from "../../../../lib/ui/Diemensions";

export function DataTimePopover(
    {refCalendar, popoverRequestHeight}: {
        refCalendar: (instance: Gtk.Calendar) => void,
        popoverRequestHeight: number,
    },
) {
    const svc = Agenda.get_default()

    const rawDateTime: Accessor<GLib.DateTime> = createPoll(
        GLib.DateTime.new_now_local(),
        1000,
        () => GLib.DateTime.new_now_local(),
    )
    const [calendarSelectedDate, setCalendarSelectedDate] = createState<GLib.DateTime>(rawDateTime.get())
    const calendarSelectedDateFormatted = calendarSelectedDate.as((date) => date.format("%A %d %b %Y")!.capitalize())

    const eventsAccessor = {
        get: () => svc.events,
        subscribe: (cb: (e: CalendarEvent[]) => void) => {
            const id = svc.connect("notify::events", () => cb(svc.events))
            return () => svc.disconnect(id)
        }
    }

    const todayEventsAccessor: Accessor<CalendarEvent[]> = new Accessor<CalendarEvent[]>(
        () => svc.events.filter(
            (event) => event.start.format("%Y%m%d")! === rawDateTime.get().format("%Y%m%d")!
        ),
        (cb: (e: CalendarEvent[]) => void) => {
            const id = svc.connect("notify::events", () => cb(todayEventsAccessor.get()))
            return () => svc.disconnect(id)
        }
    )

    const selectedDayEventsAccessor: Accessor<CalendarEvent[]> = calendarSelectedDate.as((date: GLib.DateTime) =>
        svc.events.filter((event) => event.start.format("%Y%m%d")! === date.format("%Y%m%d")!)
    )

    let calendar: Gtk.Calendar

    const markCalendar = () => {
        calendar.clear_marks();

        const currentYear = calendar.get_date().get_year();
        const currentMonth = calendar.get_date().get_month();

        eventsAccessor.get().forEach((event) => {
            const dateStr = event.start.format("%Y%m%d")!;
            const year = parseInt(dateStr.substring(0, 4));
            const month = parseInt(dateStr.substring(4, 6));
            const day = parseInt(dateStr.substring(6, 8));

            if (year === currentYear && month === currentMonth) {
                calendar.mark_day(day);
            }
        });
    }

    const updateDaySelected = () => setCalendarSelectedDate(calendar.get_date())

    onCleanup(() => {
    })

    eventsAccessor.subscribe(markCalendar);

    return (
        <box
            orientation={Gtk.Orientation.VERTICAL}
            spacing={4}
        >
            <Calendar
                ref={(instance) => {
                    calendar = instance
                    refCalendar(instance)
                }}
                markCalendar={markCalendar}
                updateDaySelected={updateDaySelected}
            />

            <scrolledwindow
                propagateNaturalWidth
                propagateNaturalHeight
                hexpand
                widthRequest={Dimensions.notificationWidth + 40}
                max_content_height={popoverRequestHeight / 2 + 24}
                max_content_width={Dimensions.notificationWidth + 40}
            >
                <box orientation={Gtk.Orientation.VERTICAL}>
                    <box>
                        <EventList events={todayEventsAccessor}/>
                    </box>
                    <box marginTop={8}/>
                    <box>
                        <EventList
                            title={calendarSelectedDateFormatted}
                            events={selectedDayEventsAccessor}
                            predicate={(eventList) => !DateTimeExt.isToday(calendarSelectedDate.get()) && eventList.length > 0}
                        />
                    </box>
                </box>
            </scrolledwindow>
        </box>
    )
}
