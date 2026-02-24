import { Volume2Icon, VolumeOffIcon } from 'lucide-react'
import { Button } from '~/components/ui/button'
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
import { AppPicker } from './app-picker'
import { ChannelIcon } from './channel-icon'
import { Meter } from './meter'
import type { Bus, Channel } from './types'
import { INPUT_DEVICES } from './types'

interface ChannelStripProps {
    channel: Channel
    onVolumeChange: (bus: Bus, value: number) => void
    onMuteToggle: (bus: Bus) => void
    onInputDeviceChange: (value: string) => void
    onApplicationsChange: (apps: string[]) => void
}

function BusColumn({
    label,
    volume,
    muted,
    onVolumeChange,
    onMuteToggle,
}: {
    label: string
    volume: number
    muted: boolean
    onVolumeChange: (value: number) => void
    onMuteToggle: () => void
}) {
    const meterValue = muted ? 0 : volume * 0.85

    return (
        <div className="flex flex-1 flex-col items-center gap-2">
            {/* Bus label */}
            <span className="text-[9px] font-semibold uppercase tracking-wider text-muted-foreground">
                {label}
            </span>

            {/* Meter + Slider */}
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

            {/* Volume readout */}
            <span className="text-[9px] font-medium tabular-nums text-muted-foreground">
                {muted ? '--' : volume}
            </span>

            {/* Mute button */}
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
    onVolumeChange,
    onMuteToggle,
    onInputDeviceChange,
    onApplicationsChange,
}: ChannelStripProps) {
    const isFullyMuted = channel.monitorMuted && channel.streamMuted
    const isMic = channel.id === 'mic'

    return (
        <div className="group flex h-full flex-col items-center gap-3 rounded-2xl border border-border bg-card p-3 transition-colors hover:border-primary/20">
            {/* Channel icon + name */}
            <div className="flex flex-col items-center gap-1.5">
                <div
                    className={cn(
                        'flex size-8 items-center justify-center rounded-lg bg-muted transition-colors',
                        !isFullyMuted && 'bg-primary/15 text-primary',
                    )}
                >
                    <ChannelIcon type={channel.icon} className="size-3.5" />
                </div>
                <span className="text-[11px] font-medium text-muted-foreground">
                    {channel.name}
                </span>
            </div>

            {/* Routing selector */}
            {isMic ? (
                <Select
                    value={channel.inputDevice ?? ''}
                    onValueChange={onInputDeviceChange}
                >
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
                    selected={channel.applications}
                    onChange={onApplicationsChange}
                />
            )}

            {/* Dual bus columns */}
            <div className="flex w-full flex-1 gap-2">
                <BusColumn
                    label="MON"
                    volume={channel.monitorVolume}
                    muted={channel.monitorMuted}
                    onVolumeChange={(v) => onVolumeChange('monitor', v)}
                    onMuteToggle={() => onMuteToggle('monitor')}
                />
                <div className="w-px shrink-0 bg-border" />
                <BusColumn
                    label="STR"
                    volume={channel.streamVolume}
                    muted={channel.streamMuted}
                    onVolumeChange={(v) => onVolumeChange('stream', v)}
                    onMuteToggle={() => onMuteToggle('stream')}
                />
            </div>
        </div>
    )
}
