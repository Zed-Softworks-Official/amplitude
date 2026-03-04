import { invoke } from '@tauri-apps/api/core'
import type { Bus, Channel } from './types'

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
