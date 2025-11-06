export type Lifecycle = {
    onStart: (cb: () => void) => () => void
    onStop: (cb: () => void) => () => void
    start: () => void
    stop: () => void
    dispose: () => void
}

export function createLifecycle(): Lifecycle {
    const startHandlers = new Set<() => void>()
    const stopHandlers = new Set<() => void>()

    const onStart = (cb: () => void) => {
        startHandlers.add(cb)
        return () => startHandlers.delete(cb)
    }

    const onStop = (cb: () => void) => {
        stopHandlers.add(cb)
        return () => stopHandlers.delete(cb)
    }

    const start = () => {
        for (const cb of startHandlers) cb()
    }

    const stop = () => {
        for (const cb of stopHandlers) cb()
    }

    const dispose = () => {
        startHandlers.clear()
        stopHandlers.clear()
    }

    return {onStart, onStop, start, stop, dispose}
}
