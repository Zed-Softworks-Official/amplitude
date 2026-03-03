import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useEffect } from 'react'

export function useSubscription<T>(event: string, callback: (data: T) => void) {
    useEffect(() => {
        let unlisten: UnlistenFn

        listen<T>(event, (e) => callback(e.payload)).then((fn) => {
            unlisten = fn
        })

        return () => unlisten?.()
    }, [event, callback])
}
