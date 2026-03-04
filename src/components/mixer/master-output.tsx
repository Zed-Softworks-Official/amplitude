import { Volume2Icon, VolumeOffIcon } from 'lucide-react'
import { useState } from 'react'
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
import { toDisplay } from '~/lib/tauri-api'
import type { Bus } from '~/lib/types'
import { Meter } from './meter'

// Placeholder output devices until backend provides them
const OUTPUT_DEVICES = [
    'Default Output',
    'Headphones',
    'Speakers',
    'HDMI Audio',
]

interface MasterOutputProps {
    label: string
    icon: React.ReactNode
    bus: Bus
    onVolumeChange: (displayVolume: number) => void
    onMuteToggle: () => void
}

export function MasterOutput({
    label,
    icon,
    bus,
    onVolumeChange,
    onMuteToggle,
}: MasterOutputProps) {
    // Frontend-only local state for output device (not yet wired to backend)
    const [outputDevice, setOutputDevice] = useState('Default Output')

    const displayVolume = toDisplay(bus.volume)
    const meterValue = bus.muted ? 0 : displayVolume * 0.9

    return (
        <div className="flex h-full flex-col items-center gap-3 rounded-2xl border border-border bg-card p-3">
            {/* Label + icon */}
            <div className="flex flex-col items-center gap-1.5">
                <div className="flex size-8 items-center justify-center rounded-lg bg-accent/15 text-accent">
                    {icon}
                </div>
                <span className="text-[11px] font-medium text-muted-foreground">
                    {label}
                </span>
            </div>

            {/* Output device selector (frontend-only) */}
            <Select value={outputDevice} onValueChange={setOutputDevice}>
                <SelectTrigger
                    size="sm"
                    className="h-7 w-full gap-1 rounded-lg px-2 text-[10px]"
                >
                    <SelectValue placeholder="Output" />
                </SelectTrigger>
                <SelectContent>
                    <SelectGroup>
                        {OUTPUT_DEVICES.map((device) => (
                            <SelectItem key={device} value={device}>
                                {device}
                            </SelectItem>
                        ))}
                    </SelectGroup>
                </SelectContent>
            </Select>

            {/* Slider + Meter */}
            <div className="flex flex-1 items-center gap-2">
                <Meter value={meterValue} />
                <Slider
                    orientation="vertical"
                    min={0}
                    max={100}
                    value={[displayVolume]}
                    onValueChange={(v) => onVolumeChange(v[0])}
                    className="h-full"
                />
            </div>

            {/* Volume readout */}
            <span className="text-[9px] font-medium tabular-nums text-muted-foreground">
                {bus.muted ? '--' : displayVolume}
            </span>

            {/* Mute */}
            <Button
                variant={bus.muted ? 'default' : 'ghost'}
                size="icon-xs"
                onClick={onMuteToggle}
                className={cn(
                    'shrink-0',
                    bus.muted &&
                        'bg-destructive/15 text-destructive hover:bg-destructive/25',
                )}
            >
                {bus.muted ? (
                    <VolumeOffIcon className="size-3" />
                ) : (
                    <Volume2Icon className="size-3" />
                )}
            </Button>
        </div>
    )
}
