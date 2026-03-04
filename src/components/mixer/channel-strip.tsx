import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import {
    GripVerticalIcon,
    Trash2Icon,
    Volume2Icon,
    VolumeOffIcon,
} from 'lucide-react'
import { useState } from 'react'
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
} from '~/components/ui/alert-dialog'
import { Button } from '~/components/ui/button'
import {
    ContextMenu,
    ContextMenuContent,
    ContextMenuItem,
    ContextMenuTrigger,
} from '~/components/ui/context-menu'
import {
    Select,
    SelectContent,
    SelectGroup,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from '~/components/ui/select'
import { Slider } from '~/components/ui/slider'
import { cn } from '~/lib/utils'
import { toDisplay } from '~/lib/tauri-api'
import type { Bus, Channel } from '~/lib/types'
import { AppPicker } from './app-picker'
import { ChannelIcon } from './channel-icon'
import { Meter } from './meter'

// Placeholder input devices until backend provides them
const INPUT_DEVICES = [
    'Default Input',
    'USB Microphone',
    'Line In',
    'Webcam Mic',
]

interface ChannelStripProps {
    channel: Channel
    buses: Bus[]
    onVolumeChange: (busId: string, displayVolume: number) => void
    onMuteToggle: (busId: string, currentMuted: boolean) => void
    onConnectionsChange: (processNames: string[]) => void
    onDelete?: () => void
}

function BusColumn({
    label,
    volume,
    muted,
    onVolumeChange,
    onMuteToggle,
}: {
    label: string
    /** Display value 0–100 */
    volume: number
    muted: boolean
    onVolumeChange: (displayVolume: number) => void
    onMuteToggle: () => void
}) {
    const meterValue = muted ? 0 : volume * 0.85

    return (
        <div className="flex flex-1 flex-col items-center gap-2">
            <span className="text-[9px] font-semibold uppercase tracking-wider text-muted-foreground">
                {label}
            </span>

            <div className="flex flex-1 items-center gap-1.5">
                <Meter value={meterValue} />
                <Slider
                    orientation="vertical"
                    min={0}
                    max={100}
                    value={[volume]}
                    onValueChange={(v) => onVolumeChange(v[0])}
                    className="h-full"
                />
            </div>

            <span className="text-[9px] font-medium tabular-nums text-muted-foreground">
                {muted ? '--' : volume}
            </span>

            <Button
                variant={muted ? 'default' : 'ghost'}
                size="icon-xs"
                onClick={onMuteToggle}
                className={cn(
                    'shrink-0',
                    muted &&
                        'bg-destructive/15 text-destructive hover:bg-destructive/25',
                )}
            >
                {muted ? (
                    <VolumeOffIcon className="size-3" />
                ) : (
                    <Volume2Icon className="size-3" />
                )}
            </Button>
        </div>
    )
}

export function ChannelStrip({
    channel,
    buses,
    onVolumeChange,
    onMuteToggle,
    onConnectionsChange,
    onDelete,
}: ChannelStripProps) {
    const isMic = channel.name.toLowerCase() === 'mic'

    // Frontend-only local state for input device (not yet wired to backend)
    const [inputDevice, setInputDevice] = useState('Default Input')
    const [deleteOpen, setDeleteOpen] = useState(false)

    const selectedConnections = channel.connections.map((c) => c.processName)

    const {
        attributes,
        listeners,
        setNodeRef,
        transform,
        transition,
        isDragging,
    } = useSortable({
        id: channel.id,
        disabled: isMic,
    })

    const style = {
        transform: CSS.Transform.toString(transform),
        transition,
        opacity: isDragging ? 0.5 : 1,
    }

    // Check if every bus send is muted
    const isFullyMuted =
        buses.length > 0 &&
        buses.every((bus) => {
            const send = channel.sends.find((s) => s.busId === bus.id)
            return send?.muted ?? false
        })

    const cardContent = (
        <div
            ref={setNodeRef}
            style={style}
            {...attributes}
            {...listeners}
            className={cn(
                'group relative flex h-full flex-col items-center gap-3 rounded-2xl border border-border bg-card p-3 transition-colors hover:border-accent/30',
                isDragging && 'z-50',
            )}
        >
            {!isMic && (
                <div className="absolute top-2 right-2 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100">
                    <GripVerticalIcon className="size-3.5" />
                </div>
            )}

            {/* Icon + name */}
            <div className="flex flex-col items-center gap-1.5">
                <div
                    className={cn(
                        'flex size-8 items-center justify-center rounded-lg bg-muted transition-colors',
                        !isFullyMuted && 'bg-accent/15 text-accent',
                    )}
                >
                    <ChannelIcon name={channel.name} className="size-3.5" />
                </div>
                <span className="text-[11px] font-medium text-muted-foreground">
                    {channel.name}
                </span>
            </div>

            {/* Routing selector */}
            {isMic ? (
                <Select value={inputDevice} onValueChange={setInputDevice}>
                    <SelectTrigger
                        size="sm"
                        className="h-7 w-full gap-1 rounded-lg px-2 text-[10px]"
                    >
                        <SelectValue placeholder="Select input" />
                    </SelectTrigger>
                    <SelectContent>
                        <SelectGroup>
                            {INPUT_DEVICES.map((device) => (
                                <SelectItem key={device} value={device}>
                                    {device}
                                </SelectItem>
                            ))}
                        </SelectGroup>
                    </SelectContent>
                </Select>
            ) : (
                <AppPicker
                    selected={selectedConnections}
                    onChange={onConnectionsChange}
                />
            )}

            {/* Dual bus columns */}
            <div className="flex w-full flex-1 gap-2">
                {buses.map((bus, i) => {
                    const send = channel.sends.find((s) => s.busId === bus.id)
                    const displayVolume = toDisplay(send?.volume ?? 0.8)
                    const muted = send?.muted ?? false

                    return (
                        <div key={bus.id} className="flex flex-1 gap-2">
                            {i > 0 && (
                                <div className="w-px shrink-0 bg-border" />
                            )}
                            <BusColumn
                                label={bus.name.slice(0, 3).toUpperCase()}
                                volume={displayVolume}
                                muted={muted}
                                onVolumeChange={(v) =>
                                    onVolumeChange(bus.id, v)
                                }
                                onMuteToggle={() =>
                                    onMuteToggle(bus.id, muted)
                                }
                            />
                        </div>
                    )
                })}
            </div>
        </div>
    )

    return (
        <>
            {!isMic ? (
                <ContextMenu>
                    <ContextMenuTrigger asChild>
                        {cardContent}
                    </ContextMenuTrigger>
                    <ContextMenuContent>
                        <ContextMenuItem
                            variant="destructive"
                            onClick={() => setDeleteOpen(true)}
                        >
                            <Trash2Icon />
                            Delete Channel
                        </ContextMenuItem>
                    </ContextMenuContent>
                </ContextMenu>
            ) : (
                cardContent
            )}

            <AlertDialog open={deleteOpen} onOpenChange={setDeleteOpen}>
                <AlertDialogContent>
                    <AlertDialogHeader>
                        <AlertDialogTitle>Delete Channel</AlertDialogTitle>
                        <AlertDialogDescription>
                            Are you sure you want to delete the {channel.name}{' '}
                            channel? This action cannot be undone.
                        </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel>Cancel</AlertDialogCancel>
                        <AlertDialogAction
                            variant="destructive"
                            onClick={() => {
                                onDelete?.()
                                setDeleteOpen(false)
                            }}
                        >
                            Delete
                        </AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </>
    )
}
