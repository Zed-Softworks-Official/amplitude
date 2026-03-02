export type ChannelId = 'mic' | 'system' | 'browser' | 'vc' | 'game' | 'music'

export type Bus = 'monitor' | 'stream'

export interface Channel {
    id: ChannelId
    name: string
    icon: ChannelId
    monitorVolume: number
    streamVolume: number
    monitorMuted: boolean
    streamMuted: boolean
    /** For Mic channel: which input device is selected */
    inputDevice?: string
    /** For non-Mic channels: which applications are routed */
    applications: string[]
}

export const CHANNEL_PRESETS: Record<
    ChannelId,
    { name: string; icon: ChannelId }
> = {
    mic: { name: 'Mic', icon: 'mic' },
    system: { name: 'System', icon: 'system' },
    browser: { name: 'Browser', icon: 'browser' },
    vc: { name: 'VC', icon: 'vc' },
    game: { name: 'Game', icon: 'game' },
    music: { name: 'Music', icon: 'music' },
}

export const ADDABLE_CHANNEL_IDS: ChannelId[] = [
    'system',
    'browser',
    'vc',
    'game',
    'music',
]

// --- Static placeholder lists for testing ---

export const INPUT_DEVICES = [
    'Default Input',
    'USB Microphone',
    'Line In',
    'Webcam Mic',
]

export const APPLICATIONS = [
    'Chrome',
    'Spotify',
    'Discord',
    'Game.exe',
    'OBS Studio',
    'Firefox',
]

export const OUTPUT_DEVICES = [
    'Default Output',
    'Headphones',
    'Speakers',
    'HDMI Audio',
]
