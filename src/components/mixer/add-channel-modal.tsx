import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogHeader,
    DialogTitle,
} from '~/components/ui/dialog'
import { cn } from '~/lib/utils'
import { addChannel } from '~/lib/tauri-api'
import { ChannelIcon } from './channel-icon'

// Frontend-only preset list. The name is passed verbatim to the backend.
const ADDABLE_PRESETS = [
    { name: 'System' },
    { name: 'Browser' },
    { name: 'VC' },
    { name: 'Game' },
    { name: 'Music' },
] as const

interface AddChannelModalProps {
    open: boolean
    onOpenChange: (open: boolean) => void
    existingChannelNames: string[]
}

export function AddChannelModal({
    open,
    onOpenChange,
    existingChannelNames,
}: AddChannelModalProps) {
    const lowerExisting = existingChannelNames.map((n) => n.toLowerCase())

    const handleAdd = (name: string) => {
        onOpenChange(false)
        addChannel(name).catch(console.error)
    }

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
                    {ADDABLE_PRESETS.map(({ name }) => {
                        const isUsed = lowerExisting.includes(name.toLowerCase())

                        return (
                            <button
                                key={name}
                                type="button"
                                disabled={isUsed}
                                onClick={() => handleAdd(name)}
                                className={cn(
                                    'flex items-center gap-3 rounded-xl border border-border bg-card p-3 text-left text-sm font-medium transition-colors',
                                    'hover:border-accent/30 hover:bg-accent/5',
                                    'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
                                    isUsed && 'pointer-events-none opacity-35',
                                )}
                            >
                                <div className="flex size-8 items-center justify-center rounded-lg bg-muted">
                                    <ChannelIcon
                                        name={name}
                                        className="size-4 text-muted-foreground"
                                    />
                                </div>
                                <div className="flex flex-col">
                                    <span className="text-foreground">
                                        {name}
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
