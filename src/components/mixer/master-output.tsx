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
import { Meter } from './meter'
import { OUTPUT_DEVICES } from './types'

interface MasterOutputProps {
    label: string
    icon: React.ReactNode
    volume: number
    muted: boolean
    outputDevice: string
    onVolumeChange: (value: number) => void
    onMuteToggle: () => void
    onOutputDeviceChange: (value: string) => void
}

export function MasterOutput({
    label,
    icon,
    volume,
    muted,
    outputDevice,
    onVolumeChange,
    onMuteToggle,
    onOutputDeviceChange,
}: MasterOutputProps) {
    const meterValue = muted ? 0 : volume * 0.9

    return (
        <div className="flex h-full flex-col items-center gap-3 rounded-2xl border border-border bg-card p-3">
            {/* Label + icon */}
            <div className="flex flex-col items-center gap-1.5">
                <div className="flex size-8 items-center justify-center rounded-lg bg-primary/15 text-primary">
                    {icon}
                </div>
                <span className="text-[11px] font-medium text-muted-foreground">
                    {label}
                </span>
            </div>

            {/* Output device selector */}
            <Select value={outputDevice} onValueChange={onOutputDeviceChange}>
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

            {/* Slider + single Meter */}
            <div className="flex flex-1 items-center gap-2">
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

            {/* Mute */}
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
