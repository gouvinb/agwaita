import GLib from "gi://GLib"
import EDataServer from "gi://EDataServer"
import ECal from "gi://ECal"
import ICalGLib from "gi://ICalGLib"
import GObject, {getter, register} from "gnim/gobject"
import {Log} from "../lib/Logger"
import {Accessor, createEffect, createState, Setter} from "ags"

export type CalendarEvent = {
    summary: string
    desc: string
    color?: string
    isAllDay: boolean
    start: GLib.DateTime
    end: GLib.DateTime
}

let instance: Agenda

@register({GTypeName: "AgendaService"})
export default class Agenda extends GObject.Object {
    declare $signals: GObject.Object.SignalSignatures & {
        "notify::events": () => void
    }

    #sourceRegistry: EDataServer.SourceRegistry | null = null
    #sources: EDataServer.Source[] = []
    #clients: ECal.Client[] = []
    #clientViews: ECal.ClientView[] = []
    #events: CalendarEvent[] = []

    #sourceRegistrySignals: number[] = []
    #clientViewSignals: Map<ECal.ClientView, number[]> = new Map()


    #eventsState: Accessor<CalendarEvent[]>
    readonly #setEventsState: Setter<CalendarEvent[]>
    #eventsSubscription: (() => void) | null = null

    constructor() {
        super()
        const [events, setEvents] = createState<CalendarEvent[]>([])

        this.#eventsState = events
        this.#setEventsState = setEvents

        createEffect(() => {
            this.#events = this.#eventsState()
            this.notify("events")
        })

    }

    @getter(Array)
    get events(): CalendarEvent[] {
        return this.#events
    }

    get sources(): EDataServer.Source[] {
        return this.#sources
    }

    get clients(): ECal.Client[] {
        return this.#clients
    }

    static get_default() {
        if (!instance) {
            instance = new Agenda()
        }
        return instance
    }

    static get_with_signals_initialized() {
        if (!instance) {
            instance = new Agenda()
            instance.initAllSignals()
        }
        return instance
    }

    initAllSignals() {
        if (!this.#sourceRegistry) {
            this.#initRegistry()
        }

        if (this.#clients.length === 0) {
            this.#updateClients()
        }
    }

    stopAllSignals() {
        this.#disconnectSourceRegistrySignals()

        if (this.#eventsSubscription) {
            this.#eventsSubscription()
            this.#eventsSubscription = null
        }

        this.#clientViews.forEach(view => {
            this.#disconnectClientViewSignals(view)
            try {
                view.stop()
                view.run_dispose()
            } catch (e) {
                Log.e("Agenda service", "Error disposing client view", e)
            }
        })
        this.#clientViews = []

        this.#clients.forEach(client => {
            try {
                client.run_dispose()
            } catch (e) {
                Log.e("Agenda service", "Error disposing client", e)
            }
        })
        this.#clients = []

        if (this.#sourceRegistry) {
            try {
                this.#sourceRegistry.run_dispose()
            } catch (e) {
                Log.e("Agenda service", "Error disposing source registry", e)
            }
            this.#sourceRegistry = null
        }

        this.#sources = []
    }

    #initRegistry() {
        try {
            this.#updateSourceRegistry()
            this.#connectSourceRegistrySignals()
            this.#updateSources()
            this.#updateClients()
        } catch (e) {
            Log.e("Agenda service", `Failed to initialize registry sources or sources`, e)
            this.#sourceRegistry = null
            this.#sources = []
        }
    }

    #connectSourceRegistrySignals() {
        if (!this.#sourceRegistry) return

        this.#sourceRegistrySignals.push(
            this.#sourceRegistry.connect("source-added", (_registry, source: EDataServer.Source) => {
                Log.i("Agenda service", `Source added: ${source.display_name}`)
                this.#updateSources()
                this.#updateClients()
            })
        )

        this.#sourceRegistrySignals.push(
            this.#sourceRegistry.connect("source-removed", (_registry, source: EDataServer.Source) => {
                Log.i("Agenda service", `Source removed: ${source.display_name}`)
                this.#updateSources()
                this.#updateClients()
            })
        )

        this.#sourceRegistrySignals.push(
            this.#sourceRegistry.connect("source-changed", (_registry, source: EDataServer.Source) => {
                Log.i("Agenda service", `Source changed: ${source.display_name}`)
                this.#updateSources()
                this.#updateClients()
            })
        )
    }

    #disconnectSourceRegistrySignals() {
        if (this.#sourceRegistry) {
            this.#sourceRegistrySignals.forEach(id => {
                this.#sourceRegistry?.disconnect(id)
            })
        }
        this.#sourceRegistrySignals = []
    }

    #updateSourceRegistry() {
        if (this.#sourceRegistry) {
            try {
                this.#sourceRegistry.run_dispose()
            } catch (e) {
                Log.e("Agenda service", "Error disposing old source registry", e)
            }
        }
        this.#sourceRegistry = EDataServer.SourceRegistry.new_sync(null)
    }

    #updateSources() {
        this.#sources = this.#sourceRegistry?.list_sources(EDataServer.SOURCE_EXTENSION_CALENDAR) ?? []
    }

    #updateClients() {
        this.#clients.forEach((client) => {
            if (this.#sources.some((src) => client.source.uid == src.uid)) return

            this.#cleanupClientViews(client)
            client.cancel_all()
        })
        this.#clients = this.#clients
            .filter((client) => this.#sources.some((src) => client.source.uid == src.uid))

        this.#sources.forEach((source) => {
            try {
                ECal.Client.connect(
                    source,
                    ECal.ClientSourceType.EVENTS,
                    1,
                    null,
                    (client) => {
                        if (client &&
                            !this.#clients.some(cli => cli.source.uid === client.source.uid)
                        ) {
                            Log.i("Agenda service", `New client: ${client.get_source().display_name}`)
                            this.#clients = [...this.#clients, client]
                            this.#createClientView(client)
                        }
                    },
                )
            } catch (e) {
                Log.e("Agenda service", `Cannot connect to client ${source.display_name}`, e)
            }
        })
    }

    #createClientView(client: ECal.Client) {
        try {
            const sexp = "(contains? \"any\" \"\")"
            const [success, view] = client.get_view_sync(sexp, null)

            if (!success || !view) {
                Log.e("Agenda service", `Cannot create view for client ${client.get_source().display_name}`)
                return
            }

            this.#clientViews.push(view)
            this.#connectClientViewSignals(view)

            // Démarrer la vue
            view.start()

            Log.i("Agenda service", `Created view for client ${client.get_source().display_name}`)
        } catch (e) {
            Log.e("Agenda service", `Error creating view for client ${client.get_source().display_name}`, e)
        }
    }

    #cleanupClientViews(client: ECal.Client) {
        const viewsToRemove = this.#clientViews.filter(view =>
            view.ref_client().source.uid === client.source.uid
        )

        viewsToRemove.forEach(view => {
            this.#disconnectClientViewSignals(view)
            try {
                view.stop()
                view.run_dispose()
            } catch (e) {
                Log.e("Agenda service", "Error disposing client view", e)
            }
        })

        this.#clientViews = this.#clientViews.filter(view =>
            view.ref_client().source.uid !== client.source.uid
        )
    }

    #connectClientViewSignals(view: ECal.ClientView) {
        const signals: number[] = []

        signals.push(
            view.connect("objects-added", () => {
                Log.i("Agenda service", `Objects added - refreshing events`)
                this.#refreshEvents()
            })
        )

        signals.push(
            view.connect("objects-modified", () => {
                Log.i("Agenda service", "Objects modified - refreshing events")
                this.#refreshEvents()
            })
        )

        signals.push(
            view.connect("objects-removed", () => {
                Log.i("Agenda service", "Objects removed - refreshing events")
                this.#refreshEvents()
            })
        )

        signals.push(
            view.connect("complete", (_view, error: GLib.Error | null) => {
                if (error) {
                    Log.e("Agenda service", `View complete with errorfor "${view.ref_client().source.display_name}"`, error)
                } else {
                    Log.i("Agenda service", `View complete - initial load done for "${view.ref_client().source.display_name}"`)
                    this.#refreshEvents()
                }
            })
        )

        this.#clientViewSignals.set(view, signals)
    }

    #disconnectClientViewSignals(view: ECal.ClientView) {
        const signals = this.#clientViewSignals.get(view)
        if (signals) {
            signals.forEach(id => {
                try {
                    view.disconnect(id)
                } catch (e) {
                    Log.e("Agenda service", "Error disconnecting signal", e)
                }
            })
            this.#clientViewSignals.delete(view)
        }
    }

    #refreshEvents() {
        this.#setEventsState(this.#listCalendarEvents())
    }

    #listCalendarEvents(): CalendarEvent[] {
        try {
            const now = GLib.DateTime.new_now_local()
            const start = now.add_years(-1)!
            const end = now.add_years(1)!
            const startTime = start.to_unix()
            const endTime = end.to_unix()

            const allEvents: CalendarEvent[] = []

            this.#clients.forEach((client) => {
                try {
                    const [_, comps] = client.get_object_list_sync("", null)

                    comps.forEach((comp) => {
                        client.generate_instances_for_object_sync(
                            comp,
                            startTime,
                            endTime,
                            null,
                            (generatedComp, instanceStart, instanceEnd) => {
                                const oneDay = ICalGLib.Duration.new_null_duration()
                                oneDay.set_days(1)

                                allEvents.push({
                                    summary: generatedComp.get_summary() ?? "",
                                    desc: generatedComp.get_description(),
                                    // @ts-expect-error untyped extension in EDataServer's d.ts
                                    color: client.source.get_extension(EDataServer.SOURCE_EXTENSION_CALENDAR).dup_color(),
                                    isAllDay: generatedComp.get_duration().as_ical_string() === oneDay.as_ical_string(),
                                    start: GLib.DateTime.new_from_unix_utc(instanceStart.as_timet())!,
                                    end: GLib.DateTime.new_from_unix_utc(instanceEnd.as_timet())!,
                                })
                                return true
                            },
                        )
                    })
                } catch (e) {
                    Log.e("Agenda service", `Cannot list or generate instances of an object with client ${client.source.display_name}`, e)
                }
            })

            return allEvents.toSorted((a, b) => a.start.compare(b.start))
        } catch (e) {
            Log.e("Agenda service", `Failed to list calendar events`, e)
            return []
        }
    }
}
