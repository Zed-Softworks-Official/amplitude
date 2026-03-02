import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogHeader,
    DialogTitle,
} from '~/components/ui/dialog'
import { cn } from '~/lib/utils'
import { ChannelIcon } from './channel-icon'
import type { ChannelId } from './types'
import { ADDABLE_CHANNEL_IDS, CHANNEL_PRESETS } from './types'

interface AddChannelModalProps {
    open: boolean
    onOpenChange: (open: boolean) => void
    existingChannelIds: ChannelId[]
    onAddChannel: (id: ChannelId) => void
}

export function AddChannelModal({
    open,
    onOpenChange,
    existingChannelIds,
    onAddChannel,
}: AddChannelModalProps) {
    const availableChannels = ADDABLE_CHANNEL_IDS.filter(
        (id) => !existingChannelIds.includes(id),
    )

    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent className="sm:max-w-sm">
                <DialogHeader>
                    <DialogTitle>Add Channel</DialogTitle>
                    <DialogDescription>
                        Choose an audio source to add to your mixer.
                    </DialogDescription>
                </DialogHeader>
                <div className="grid grid-cols-2 gap-2">
                    {ADDABLE_CHANNEL_IDS.map((id) => {
                        const preset = CHANNEL_PRESETS[id]
                        const isUsed = !availableChannels.includes(id)

                        return (
                            <button
                                key={id}
                                type="button"
                                disabled={isUsed}
                                onClick={() => {
                                    onAddChannel(id)
                                    onOpenChange(false)
                                }}
                                className={cn(
                                    'flex items-center gap-3 rounded-xl border border-border bg-card p-3 text-left text-sm font-medium transition-colors',
                                    'hover:border-accent/30 hover:bg-accent/5',
                                    'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
                                    isUsed && 'pointer-events-none opacity-35',
                                )}
                            >
                                <div className="flex size-8 items-center justify-center rounded-lg bg-muted">
                                    <ChannelIcon
                                        type={id}
                                        className="size-4 text-muted-foreground"
                                    />
                                </div>
                                <div className="flex flex-col">
                                    <span className="text-foreground">
                                        {preset.name}
                                    </span>
                                    {isUsed && (
                                        <span className="text-[10px] text-muted-foreground">
                                            Already added
                                        </span>
                                    )}
                                </div>
                            </button>
                        )
                    })}
                </div>
            </DialogContent>
        </Dialog>
    )
}
