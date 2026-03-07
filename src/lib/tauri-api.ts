import { invoke } from '@tauri-apps/api/core'
import type { Bus, Channel, NodeInfo } from './types'

// ---------------------------------------------------------------------------
// Volume conversion helpers
// Backend: f32 0–1  |  Frontend display: 0–100
// ---------------------------------------------------------------------------

export function toDisplay(backendVolume: number): number {
    return Math.round(backendVolume * 100)
}

export function fromDisplay(displayVolume: number): number {
    return Math.max(0, Math.min(1, displayVolume / 100))
}

// ---------------------------------------------------------------------------
// AppState event payload shape
// ---------------------------------------------------------------------------

export interface AppStatePayload {
    channels: Channel[]
    buses: Bus[]
}

// ---------------------------------------------------------------------------
// Channel commands
// ---------------------------------------------------------------------------

export function getChannels(): Promise<Channel[]> {
    return invoke('get_channels')
}

export function addChannel(name: string): Promise<Channel> {
    return invoke('add_channel', { name })
}

export function deleteChannel(id: string): Promise<void> {
    return invoke('delete_channel', { id })
}

export function reorderChannels(order: string[]): Promise<void> {
    return invoke('reorder_channels', { order })
}

/**
 * Updates volume and/or muted state for a channel's send to a specific bus.
 * Volume is expected in display range (0–100) and is converted before sending.
 */
export function updateChannelSend(
    channelId: string,
    busId: string,
    opts: { volume?: number; muted?: boolean },
): Promise<void> {
    return invoke('update_channel_send', {
        channelId,
        busId,
        volume: opts.volume !== undefined ? fromDisplay(opts.volume) : null,
        muted: opts.muted !== undefined ? opts.muted : null,
    })
}

export function updateChannelConnections(
    channelId: string,
    processNames: string[],
): Promise<void> {
    return invoke('update_channel_connections', { channelId, processNames })
}

// ---------------------------------------------------------------------------
// Bus commands
// ---------------------------------------------------------------------------

export function getBuses(): Promise<Bus[]> {
    return invoke('get_buses')
}

/**
 * Updates volume and/or muted state for a bus.
 * Volume is expected in display range (0–100) and is converted before sending.
 */
export function updateBus(
    busId: string,
    opts: { volume?: number; muted?: boolean },
): Promise<void> {
    return invoke('update_bus', {
        busId,
        volume: opts.volume !== undefined ? fromDisplay(opts.volume) : null,
        muted: opts.muted !== undefined ? opts.muted : null,
    })
}

// ---------------------------------------------------------------------------
// Node commands
// ---------------------------------------------------------------------------

export function getNodes(): Promise<NodeInfo[]> {
    return invoke('get_nodes')
}

// ---------------------------------------------------------------------------
// Routing commands
// ---------------------------------------------------------------------------

/**
 * Route a physical input node (mic, line-in, etc.) into a channel's virtual
 * sink. Replaces any previously set input link for that channel.
 * `inputNodeId` is the PipeWire global ID of the source node.
 */
export function setChannelInput(
    channelId: string,
    inputNodeId: number,
): Promise<void> {
    return invoke('set_channel_input', { channelId, inputNodeId })
}

/**
 * Route the monitor output of a bus's virtual sink to a physical output
 * device. Replaces any previously set output link for that bus.
 * `outputNodeId` is the PipeWire global ID of the physical sink node.
 */
export function setBusOutput(
    busId: string,
    outputNodeId: number,
): Promise<void> {
    return invoke('set_bus_output', { busId, outputNodeId })
}
