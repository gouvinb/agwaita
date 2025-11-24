import GLib from "gi://GLib"

/**
 * @type {Readonly<Record<LogLevelKey, string>>}
 * L'objet 'LogLevel' mappe le niveau (clé) à son code court (valeur).
 */
const LogLevel = {
    CRITICAL: {
        key: "C",
        priority: 0
    },
    ERROR: {
        key: "E",
        priority: 1
    },
    WARNING: {
        key: "W",
        priority: 2
    },
    INFO: {
        key: "I",
        priority: 3
    },
    DEBUG: {
        key: "D",
        priority: 4
    },
} as const

const AGWAITA_LOG_LEVEL_ENV_KEY = "AGWAITA_LOG_LEVEL"

type LogLevelKey = keyof typeof LogLevel
type LogLevelValue = typeof LogLevel[LogLevelKey]

export const Log = new class {
    c(tag: string, msg: string, err?: Error | unknown | null | undefined) {
        this.printLog(LogLevel.CRITICAL, tag, msg, err)
    }

    e(tag: string, msg: string, err?: Error | unknown | null | undefined) {
        this.printLog(LogLevel.ERROR, tag, msg, err)
    }

    w(tag: string, msg: string, err?: Error | unknown | null | undefined) {
        this.printLog(LogLevel.WARNING, tag, msg, err)
    }

    i(tag: string, msg: string) {
        this.printLog(LogLevel.INFO, tag, msg)
    }

    d(tag: string, msg: string) {
        this.printLog(LogLevel.DEBUG, tag, msg)
    }

    private get currentAgwaitaLogLevel() {
        let variable = GLib.getenv(AGWAITA_LOG_LEVEL_ENV_KEY)?.toLocaleUpperCase() as LogLevelKey
        if (LogLevel[variable] == null) variable = "INFO" as LogLevelKey
        return LogLevel[variable]
    }

    private printLog(level: LogLevelValue, tag: string, msg: string, err?: Error | unknown | null | undefined) {
        if (this.currentAgwaitaLogLevel.priority < level.priority) return

        const date = new Date()

        const log = `${date.toLocaleString()}|${level.key}|${tag.substring(0, 25).padEnd(25, "_")}|${msg}${err ? `: ${err}` : ""}`

        if (level.key === "C" || level.key === "E" || level.key === "W") {
            printerr(log)
        } else if (level.key === "I" || level.key === "D") {
            print(log)
        }
    }
}
