import {
    closestCenter,
    DndContext,
    type DragEndEvent,
    KeyboardSensor,
    PointerSensor,
    useSensor,
    useSensors,
} from '@dnd-kit/core'
import {
    arrayMove,
    horizontalListSortingStrategy,
    SortableContext,
} from '@dnd-kit/sortable'
import { PlusIcon, RadioIcon, SpeakerIcon } from 'lucide-react'
import { useCallback, useEffect, useState } from 'react'
import { Separator } from '~/components/ui/separator'
import { useSubscription } from '~/hooks/use-subscription'
import {
    deleteChannel,
    getBuses,
    getChannels,
    reorderChannels,
    type AppStatePayload,
    updateBus,
    updateChannelConnections,
    updateChannelSend,
} from '~/lib/tauri-api'
import type { Bus, Channel } from '~/lib/types'
import { AddChannelModal } from './add-channel-modal'
import { ChannelStrip } from './channel-strip'
import { MasterOutput } from './master-output'

export function MixerApp() {
    const [channels, setChannels] = useState<Channel[]>([])
    const [buses, setBuses] = useState<Bus[]>([])
    const [addModalOpen, setAddModalOpen] = useState(false)

    // ---------------------------------------------------------------------------
    // Initial load
    // ---------------------------------------------------------------------------

    useEffect(() => {
        getChannels().then(setChannels).catch(console.error)
        getBuses().then(setBuses).catch(console.error)
    }, [])

    // ---------------------------------------------------------------------------
    // Live updates via Tauri event
    // ---------------------------------------------------------------------------

    const handleAppStateChanged = useCallback((payload: AppStatePayload) => {
        setChannels(payload.channels)
        setBuses(payload.buses)
    }, [])

    useSubscription<AppStatePayload>('appstate-changed', handleAppStateChanged)

    // ---------------------------------------------------------------------------
    // DnD
    // ---------------------------------------------------------------------------

    const sensors = useSensors(
        useSensor(PointerSensor, {
            activationConstraint: { distance: 8 },
        }),
        useSensor(KeyboardSensor),
    )

    const handleDragEnd = (event: DragEndEvent) => {
        const { active, over } = event
        if (!over || active.id === over.id) return

        const oldIndex = channels.findIndex((ch) => ch.id === active.id)
        const newIndex = channels.findIndex((ch) => ch.id === over.id)

        // Prevent moving mic (index 0) or moving anything to index 0
        if (oldIndex === 0 || newIndex === 0) return

        const reordered = arrayMove(channels, oldIndex, newIndex)
        setChannels(reordered)
        reorderChannels(reordered.map((ch) => ch.id)).catch(console.error)
    }

    // ---------------------------------------------------------------------------
    // Handlers
    // ---------------------------------------------------------------------------

    const handleVolumeChange = (
        channelId: string,
        busId: string,
        displayVolume: number,
    ) => {
        updateChannelSend(channelId, busId, { volume: displayVolume }).catch(
            console.error,
        )
    }

    const handleMuteToggle = (channelId: string, busId: string, currentMuted: boolean) => {
        updateChannelSend(channelId, busId, { muted: !currentMuted }).catch(
            console.error,
        )
    }

    const handleConnectionsChange = (channelId: string, processNames: string[]) => {
        updateChannelConnections(channelId, processNames).catch(console.error)
    }

    const handleDeleteChannel = (id: string) => {
        deleteChannel(id).catch(console.error)
    }

    const handleBusVolumeChange = (busId: string, displayVolume: number) => {
        updateBus(busId, { volume: displayVolume }).catch(console.error)
    }

    const handleBusMuteToggle = (busId: string, currentMuted: boolean) => {
        updateBus(busId, { muted: !currentMuted }).catch(console.error)
    }

    // ---------------------------------------------------------------------------
    // Derived state
    // ---------------------------------------------------------------------------

    const monitorBus = buses.find((b) => b.name === 'monitor')
    const streamBus = buses.find((b) => b.name === 'stream')

    const existingChannelNames = channels.map((ch) => ch.name)

    return (
        <div className="flex h-screen w-screen overflow-hidden">
            {/* Channel area */}
            <main className="flex flex-1 items-stretch gap-3 overflow-x-auto p-4">
                <DndContext
                    sensors={sensors}
                    collisionDetection={closestCenter}
                    onDragEnd={handleDragEnd}
                >
                    <SortableContext
                        items={channels.map((ch) => ch.id)}
                        strategy={horizontalListSortingStrategy}
                    >
                        {channels.map((channel) => (
                            <div
                                key={channel.id}
                                className="w-[160px] shrink-0"
                            >
                                <ChannelStrip
                                    channel={channel}
                                    buses={buses}
                                    onVolumeChange={(busId, v) =>
                                        handleVolumeChange(channel.id, busId, v)
                                    }
                                    onMuteToggle={(busId, muted) =>
                                        handleMuteToggle(channel.id, busId, muted)
                                    }
                                    onConnectionsChange={(names) =>
                                        handleConnectionsChange(channel.id, names)
                                    }
                                    onDelete={() =>
                                        handleDeleteChannel(channel.id)
                                    }
                                />
                            </div>
                        ))}
                    </SortableContext>
                </DndContext>

                {/* Add channel button */}
                <button
                    type="button"
                    onClick={() => setAddModalOpen(true)}
                    className="flex w-[160px] shrink-0 flex-col items-center justify-center gap-2 rounded-2xl border border-dashed border-border text-muted-foreground transition-colors hover:border-accent/30 hover:text-foreground"
                >
                    <div className="flex size-9 items-center justify-center rounded-xl bg-muted">
                        <PlusIcon className="size-4" />
                    </div>
                    <span className="text-xs font-medium">Add Channel</span>
                </button>
            </main>

            <Separator orientation="vertical" />

            {/* Master outputs */}
            <aside className="flex shrink-0 items-stretch gap-3 p-4">
                {monitorBus && (
                    <MasterOutput
                        label="Monitor"
                        icon={<SpeakerIcon className="size-3.5" />}
                        bus={monitorBus}
                        onVolumeChange={(v) =>
                            handleBusVolumeChange(monitorBus.id, v)
                        }
                        onMuteToggle={() =>
                            handleBusMuteToggle(monitorBus.id, monitorBus.muted)
                        }
                    />
                )}
                {streamBus && (
                    <MasterOutput
                        label="Stream"
                        icon={<RadioIcon className="size-3.5" />}
                        bus={streamBus}
                        onVolumeChange={(v) =>
                            handleBusVolumeChange(streamBus.id, v)
                        }
                        onMuteToggle={() =>
                            handleBusMuteToggle(streamBus.id, streamBus.muted)
                        }
                    />
                )}
            </aside>

            <AddChannelModal
                open={addModalOpen}
                onOpenChange={setAddModalOpen}
                existingChannelNames={existingChannelNames}
            />
        </div>
    )
}
