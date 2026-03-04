import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useEffect, useRef } from 'react'

export function useSubscription<T>(event: string, callback: (data: T) => void) {
    // Keep latest callback in a ref so the effect never needs to re-run just
    // because the caller's function reference changed.
    const callbackRef = useRef(callback)
    callbackRef.current = callback

    useEffect(() => {
        let unlisten: UnlistenFn

        listen<T>(event, (e) => callbackRef.current(e.payload)).then((fn) => {
            unlisten = fn
        })

        return () => unlisten?.()
    }, [event])
}
