import { PlusIcon, RadioIcon, SpeakerIcon } from 'lucide-react'
import { useState } from 'react'
import { Separator } from '~/components/ui/separator'
import { AddChannelModal } from './add-channel-modal'
import { ChannelStrip } from './channel-strip'
import { MasterOutput } from './master-output'
import type { Bus, Channel, ChannelId } from './types'
import { CHANNEL_PRESETS } from './types'

function createChannel(id: ChannelId): Channel {
    const preset = CHANNEL_PRESETS[id]
    return {
        id,
        name: preset.name,
        icon: preset.icon,
        monitorVolume: 75,
        streamVolume: 75,
        monitorMuted: false,
        streamMuted: false,
        inputDevice: id === 'mic' ? 'Default Input' : undefined,
        applications: [],
    }
}

export function MixerApp() {
    // Mic is always present and first
    const [channels, setChannels] = useState<Channel[]>([createChannel('mic')])
    const [addModalOpen, setAddModalOpen] = useState(false)

    // Master outputs with output device selection
    const [monitor, setMonitor] = useState({
        volume: 80,
        muted: false,
        outputDevice: 'Default Output',
    })
    const [stream, setStream] = useState({
        volume: 80,
        muted: false,
        outputDevice: 'Default Output',
    })

    const handleVolumeChange = (id: ChannelId, bus: Bus, value: number) => {
        setChannels((prev) =>
            prev.map((ch) => {
                if (ch.id !== id) return ch
                if (bus === 'monitor') return { ...ch, monitorVolume: value }
                return { ...ch, streamVolume: value }
            }),
        )
    }

    const handleMuteToggle = (id: ChannelId, bus: Bus) => {
        setChannels((prev) =>
            prev.map((ch) => {
                if (ch.id !== id) return ch
                if (bus === 'monitor')
                    return { ...ch, monitorMuted: !ch.monitorMuted }
                return { ...ch, streamMuted: !ch.streamMuted }
            }),
        )
    }

    const handleInputDeviceChange = (id: ChannelId, value: string) => {
        setChannels((prev) =>
            prev.map((ch) => {
                if (ch.id !== id) return ch
                return { ...ch, inputDevice: value }
            }),
        )
    }

    const handleApplicationsChange = (id: ChannelId, apps: string[]) => {
        setChannels((prev) =>
            prev.map((ch) => {
                if (ch.id !== id) return ch
                return { ...ch, applications: apps }
            }),
        )
    }

    const handleAddChannel = (id: ChannelId) => {
        setChannels((prev) => [...prev, createChannel(id)])
    }

    const existingIds = channels.map((ch) => ch.id)
    const allPresetsUsed = existingIds.length >= 5

    return (
        <div className="flex h-screen w-screen overflow-hidden">
            {/* Channel area */}
            <main className="flex flex-1 items-stretch gap-3 overflow-x-auto p-4">
                {/* Channel strips */}
                {channels.map((channel) => (
                    <div key={channel.id} className="w-[160px] shrink-0">
                        <ChannelStrip
                            channel={channel}
                            onVolumeChange={(bus, v) =>
                                handleVolumeChange(channel.id, bus, v)
                            }
                            onMuteToggle={(bus) =>
                                handleMuteToggle(channel.id, bus)
                            }
                            onInputDeviceChange={(value) =>
                                handleInputDeviceChange(channel.id, value)
                            }
                            onApplicationsChange={(apps) =>
                                handleApplicationsChange(channel.id, apps)
                            }
                        />
                    </div>
                ))}

                {/* Add channel button */}
                {!allPresetsUsed && (
                    <button
                        type="button"
                        onClick={() => setAddModalOpen(true)}
                        className="flex w-[160px] shrink-0 flex-col items-center justify-center gap-2 rounded-2xl border border-dashed border-border text-muted-foreground transition-colors hover:border-primary/30 hover:text-foreground"
                    >
                        <div className="flex size-9 items-center justify-center rounded-xl bg-muted">
                            <PlusIcon className="size-4" />
                        </div>
                        <span className="text-xs font-medium">Add Channel</span>
                    </button>
                )}
            </main>

            {/* Separator */}
            <Separator orientation="vertical" />

            {/* Master outputs */}
            <aside className="flex shrink-0 items-stretch gap-3 p-4">
                <MasterOutput
                    label="Monitor"
                    icon={<SpeakerIcon className="size-3.5" />}
                    volume={monitor.volume}
                    muted={monitor.muted}
                    outputDevice={monitor.outputDevice}
                    onVolumeChange={(v) =>
                        setMonitor((prev) => ({ ...prev, volume: v }))
                    }
                    onMuteToggle={() =>
                        setMonitor((prev) => ({
                            ...prev,
                            muted: !prev.muted,
                        }))
                    }
                    onOutputDeviceChange={(v) =>
                        setMonitor((prev) => ({ ...prev, outputDevice: v }))
                    }
                />
                <MasterOutput
                    label="Stream"
                    icon={<RadioIcon className="size-3.5" />}
                    volume={stream.volume}
                    muted={stream.muted}
                    outputDevice={stream.outputDevice}
                    onVolumeChange={(v) =>
                        setStream((prev) => ({ ...prev, volume: v }))
                    }
                    onMuteToggle={() =>
                        setStream((prev) => ({
                            ...prev,
                            muted: !prev.muted,
                        }))
                    }
                    onOutputDeviceChange={(v) =>
                        setStream((prev) => ({ ...prev, outputDevice: v }))
                    }
                />
            </aside>

            {/* Add channel modal */}
            <AddChannelModal
                open={addModalOpen}
                onOpenChange={setAddModalOpen}
                existingChannelIds={existingIds}
                onAddChannel={handleAddChannel}
            />
        </div>
    )
}
