import {Accessor, createState, onCleanup} from "ags"
import {Gtk} from "ags/gtk4"
import {interval, Timer} from "ags/time"
import GLib from "gi://GLib"
import Agenda, {CalendarEvent} from "../../../../services/Agenda"
import {Calendar} from "./Calendar";
import {EventList} from "./EventList";
import {DateTimeExt} from "../../../../lib/extension/GLibDateTime";
import {Dimensions} from "../../../../lib/ui/Diemensions";
import {Lifecycle} from "../../../../lib/Lifecyle";

export function DataTimePopover(
    {parentLifecycle, popoverRequestHeight}: {
        parentLifecycle: Lifecycle,
        popoverRequestHeight: number,
    },
) {
    const agenda = Agenda.get_default()

    const [rawDateTime, setRawDateTime] = createState(GLib.DateTime.new_now_local())

    let rawDateTimeTimer : Timer | null = null

    const [calendarSelectedDate, setCalendarSelectedDate] = createState<GLib.DateTime>(rawDateTime.get())
    const calendarSelectedDateFormatted = calendarSelectedDate.as((date) => date.format("%A %d %b %Y")!.capitalize())

    const eventsAccessor = {
        get: () => agenda.events,
        subscribe: (cb: (e: CalendarEvent[]) => void) => {
            const id = agenda.connect("notify::events", () => cb(agenda.events))
            return () => agenda.disconnect(id)
        }
    }

    const todayEventsAccessor: Accessor<CalendarEvent[]> = new Accessor<CalendarEvent[]>(
        () => agenda.events.filter(
            (event) => event.start.format("%Y%m%d")! === (rawDateTime?.get().format("%Y%m%d") ?? "")
        ),
        (cb: (e: CalendarEvent[]) => void) => {
            const id = agenda.connect("notify::events", () => cb(todayEventsAccessor.get()))
            return () => agenda.disconnect(id)
        }
    )

    const selectedDayEventsAccessor: Accessor<CalendarEvent[]> = calendarSelectedDate.as((date: GLib.DateTime) =>
        agenda.events.filter((event) => event.start.format("%Y%m%d")! === date.format("%Y%m%d")!)
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

    const updateDaySelected = (calendar : Gtk.Calendar) => setCalendarSelectedDate(calendar.get_date())
        parentLifecycle.onStart(() => {
            rawDateTimeTimer = interval(
                1000,
                () => setRawDateTime(GLib.DateTime.new_now_local()),
            )
            agenda.initAllTimer()
        })
        parentLifecycle.onStop(() => {
            rawDateTimeTimer?.cancel()
            rawDateTimeTimer = null
            agenda.stopAllTimer()
        })

    onCleanup(() => {
    })

    eventsAccessor.subscribe(markCalendar);

    return (
        <box
            orientation={Gtk.Orientation.VERTICAL}
            spacing={Dimensions.smallSpacing}
        >
            <Calendar
                parentLifecycle={parentLifecycle}
                ref={(instance) => {
                    calendar = instance
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
