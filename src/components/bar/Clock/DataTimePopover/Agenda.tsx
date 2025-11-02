import {Accessor, createState, onCleanup} from "ags"
import {Gtk} from "ags/gtk4"
import {createPoll, interval} from "ags/time"
import GLib from "gi://GLib"
import EDataServer from "gi://EDataServer"
import ECal from "gi://ECal"
import ICalGLib from "gi://ICalGLib"
import {Calendar} from "./Calendar";
import {EventList} from "./EventList";
import {DateTimeExt} from "../../../../lib/extension/GLibDateTime";
import {notificationWidth} from "../../../notifications/Notification";

export type CalendarEvent = {
    summary: string
    desc: string
    color?: string
    isAllDay: boolean
    start: GLib.DateTime
    end: GLib.DateTime
}

export function Agenda(
    {refCalendar, popoverRequestHeight}: {
        refCalendar: (instance: Gtk.Calendar) => void,
        popoverRequestHeight: number,
    },
) {
    const rawDateTime: Accessor<GLib.DateTime> = createPoll(
        GLib.DateTime.new_now_local(),
        1000,
        () => GLib.DateTime.new_now_local(),
    )
    const [calendarSelectedDate, setCalendarSelectedDate] = createState<GLib.DateTime>(rawDateTime.get())
    const calendarSelectedDateFormatted = calendarSelectedDate.as((date) => date.format("%A %d %b %Y")!.capitalize())

    const [clients, setClients] = createState<Array<ECal.Client>>(new Array<ECal.Client>())

    const registry = (() => {
        try {
            return EDataServer.SourceRegistry.new_sync(null)
        } catch (e) {
            printerr(e)
            return null
        }
    })()
    const sources = registry?.list_sources(EDataServer.SOURCE_EXTENSION_CALENDAR) ?? []
    const eventsAccessor = createPoll<CalendarEvent[]>([], (sources.length + 1) * 1_000, listCalendarEvents)
    const todayEventsAccessor = eventsAccessor.as((events) =>
        events.filter((event) => event.start.format("%Y%m%d")! == rawDateTime.get().format("%Y%m%d")!)
    )
    const selectedDayEventsAccessor: Accessor<CalendarEvent[]> = calendarSelectedDate.as((date: GLib.DateTime) =>
        eventsAccessor.get().filter((event) => event.start.format("%Y%m%d")! == date.format("%Y%m%d")!)
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

    const updateDaySelected = () => {
        setCalendarSelectedDate(calendar.get_date());
    }

    function updateClients() {
        setClients([])
        sources
            .forEach((source) => {
                return ECal.Client.connect(
                    source,
                    ECal.ClientSourceType.EVENTS,
                    1,
                    null,
                    (client) => {
                        if (client != null) {
                            setClients((value: ECal.Client[]) => [...value, client])
                        }
                    }
                )
            })
    }

    function listCalendarEvents(): CalendarEvent[] {
        try {
            const now = rawDateTime.get();
            const start = now.add_years(-1)!;
            const end = now.add_years(1)!;

            const startTime = start.to_unix();
            const endTime = end.to_unix();

            const allEvents: CalendarEvent[] = [];

            clients.get().forEach((client: ECal.Client) => {
                try {
                    const [_, comps] = client.get_object_list_sync("", null);

                    comps.forEach((comp) => {
                        client.generate_instances_for_object_sync(
                            comp,
                            startTime,
                            endTime,
                            null,
                            (comp, instanceStart, instanceEnd) => {
                                const oneDay = ICalGLib.Duration.new_null_duration()
                                oneDay.set_days(1)

                                allEvents.push({
                                    summary: comp.get_summary() ?? "",
                                    desc: comp.get_description(),
                                    color:
                                    // @ts-expect-error: extension non typée dans les d.ts de EDataServer
                                        client.source.get_extension(EDataServer.SOURCE_EXTENSION_CALENDAR).dup_color(),
                                    isAllDay: comp.get_duration().as_ical_string() == oneDay.as_ical_string(),
                                    start: GLib.DateTime.new_from_unix_utc(instanceStart.as_timet())!,
                                    end: GLib.DateTime.new_from_unix_utc(instanceEnd.as_timet())!
                                });
                                return true;
                            },
                        );
                    });
                } catch (e) {
                    printerr(e);
                }
            });

            return allEvents
                .toSorted((a, b) => a.start.compare(b.start));
        } catch (e) {
            printerr(e);
            return [];
        }
    }

    onCleanup(() => {
    })

    eventsAccessor.subscribe(markCalendar);

    interval((sources.length + 1) * 1000, () => {
        updateClients();
    });

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
                widthRequest={notificationWidth + 40}
                max_content_height={popoverRequestHeight / 2 + 24}
                max_content_width={notificationWidth +40}
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
