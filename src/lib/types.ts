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
