// ---------------------------------------------------------------------------
// MediaClass — mirrors the Rust enum with serde tag = "type", content = "value"
// ---------------------------------------------------------------------------

export type MediaClass =
    | { type: 'audioSink' }
    | { type: 'audioSource' }
    | { type: 'streamOutputAudio' }
    | { type: 'streamInputAudio' }
    | { type: 'other'; value: string }

// ---------------------------------------------------------------------------
// NodeInfo — a single PipeWire node
// ---------------------------------------------------------------------------

export interface NodeInfo {
    id: number
    name: string
    description: string | null
    appName: string | null
    appBinary: string | null
    mediaClass: MediaClass | null
    icon: string | null
    isAmplitudeVirtual: boolean
}

// ---------------------------------------------------------------------------
// Channels / Buses
// ---------------------------------------------------------------------------

export interface Channel {
    id: string
    name: string
    sends: Send[]
    connections: Connection[]
}

export interface Send {
    busId: string
    volume: number
    muted: boolean
}

export interface Connection {
    processId: number
    processName: string
}

export interface Bus {
    id: string
    name: string
    volume: number
    muted: boolean
}
