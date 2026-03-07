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
import { toDisplay } from '~/lib/tauri-api'
import type { Bus, NodeInfo } from '~/lib/types'
import { cn } from '~/lib/utils'
import { Meter } from './meter'

interface MasterOutputProps {
    label: string
    icon: React.ReactNode
    bus: Bus
    nodes: NodeInfo[]
    onVolumeChange: (displayVolume: number) => void
    onMuteToggle: () => void
}

export function MasterOutput({
    label,
    icon,
    bus,
    nodes,
    onVolumeChange,
    onMuteToggle,
}: MasterOutputProps) {
    const [outputDevice, setOutputDevice] = useState<string>('')

    // Physical audio outputs only — exclude our own virtual sinks.
    const outputNodes = nodes.filter(
        (n) => !n.isAmplitudeVirtual && n.mediaClass?.type === 'audioSink',
    )

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

            {/* Output device selector */}
            <Select value={outputDevice} onValueChange={setOutputDevice}>
                <SelectTrigger
                    size="sm"
                    className="h-7 w-full gap-1 rounded-lg px-2 text-[10px]"
                >
                    <SelectValue placeholder="Output" />
                </SelectTrigger>
                <SelectContent>
                    <SelectGroup>
                        {outputNodes.length === 0 ? (
                            <SelectItem value="" disabled>
                                No outputs found
                            </SelectItem>
                        ) : (
                            outputNodes.map((node) => (
                                <SelectItem
                                    key={node.id}
                                    value={String(node.id)}
                                >
                                    {node.description ?? node.name}
                                </SelectItem>
                            ))
                        )}
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
