-- Native Neo Geo soft DIP override, not a ROM patch or a replacement NVRAM.
-- Layout: https://www.ajworld.net/neogeodev/beginner/#SoftwareDips
-- BIOS_GAME_DIP is the 16-byte BIOS ABI at 0x10fd84. Cartridge metadata
-- supplies the option's position and values; these differ between games.
local adapter = {}

function adapter.find_blood(read, pointer)
    local function byte(offset) return read(pointer + offset) end
    local function label(offset)
        local chars = {}
        for i = 0, 11 do chars[#chars + 1] = string.char(byte(offset + i)) end
        return table.concat(chars):match('^%s*(.-)%s*$'):upper()
    end
    local cursor = 0x20
    -- Each enabled timed/counter option has a title before the choice labels.
    for _, offset in ipairs({0x10, 0x12, 0x14, 0x15}) do
        if byte(offset) ~= 0xff then cursor = cursor + 12 end
    end
    local found
    for index = 0, 9 do
        local descriptor = byte(0x16 + index)
        local count, default = descriptor & 0x0f, descriptor >> 4
        if count == 0 then
            if descriptor ~= 0 then return nil end
        else
            if default >= count then return nil end
            local name = label(cursor)
            cursor = cursor + 12
            local on, off
            for value = 0, count - 1 do
                local choice = label(cursor)
                if choice == 'ON' then on = value end
                if choice == 'OFF' then off = value end
                cursor = cursor + 12
            end
            if name == 'BLOOD' then
                if found or count ~= 2 or on == nil or off == nil then return nil end
                found = {offset = 6 + index, on = on, off = off}
            end
        end
    end
    return found
end

function adapter.start(enabled)
    local machine = manager.machine
    local function unavailable(reason)
        emu.print_error('LUNCHBOX_ARCADE_BLOOD_UNAVAILABLE: ' .. reason)
        machine:popmessage('Lunchbox: Blood preference could not be applied. ' .. reason)
    end
    local cpu = machine.devices[':maincpu']
    local rom = machine.memory.regions[':cslot1:maincpu']
    if not cpu or not rom or rom.endianness ~= 'big' or rom.bitwidth ~= 16 then
        unavailable('A Neo Geo MVS cartridge is required.')
        return
    end
    local space = cpu.spaces['program']
    -- read_u16 honours the ROM region's endian declaration, unlike raw read().
    local function read(address)
        if address < 0 or address >= rom.size then error('Invalid soft DIP address') end
        local word = rom:read_u16(address & ~1)
        return address % 2 == 0 and (word >> 8) or (word & 0xff)
    end
    local signature = ''
    for address = 0x100, 0x106 do signature = signature .. string.char(read(address)) end
    -- The following byte is not always zero on cartridges (Metal Slug 2
    -- uses 0x10). The MVS cartridge device and signature identify hardware.
    if signature ~= 'NEO-GEO' then
        unavailable('The cartridge header is not supported.')
        return
    end
    local function long(address)
        return (read(address) << 24) | (read(address+1) << 16)
            | (read(address+2) << 8) | read(address+3)
    end
    -- The cartridge's first bank is mapped at 0x200000, following the fixed
    -- 1 MiB. Read the decoded ROM region, not the current bank restored by a
    -- save state. No bank switching or writes to ROM are needed.
    local function table_offset(pointer)
        if pointer >= 0x200000 and pointer < 0x300000 then return pointer - 0x100000 end
        if pointer >= 0x200 and pointer < 0x100000 then return pointer end
        error('Unsupported soft DIP bank')
    end
    local ok, dip = pcall(function()
        local usa = adapter.find_blood(read, table_offset(long(0x11a)))
        local europe = adapter.find_blood(read, table_offset(long(0x11e)))
        if not usa or not europe or usa.offset ~= europe.offset
            or usa.on ~= europe.on or usa.off ~= europe.off then return nil end
        return usa
    end)
    if not ok or not dip then
        unavailable('No unambiguous native Blood on/off option was found.')
        return
    end
    local address, value = 0x10fd84 + dip.offset, enabled and dip.on or dip.off
    local announced = false
    local function apply()
        -- Do not interfere with BIOS RAM tests, initialization, or the service
        -- menu. Only change the one documented option while the game runs.
        local request, mode = space:read_u8(0x10fdae), space:read_u8(0x10fdaf)
        if (request ~= 2 and request ~= 3) or (mode ~= 1 and mode ~= 2) then return end
        if space:read_u8(0x10fd82) ~= 0x80 then
            if not announced then unavailable('Select an arcade (MVS) BIOS, not AES.') end
            announced = true
            return
        end
        if space:read_u8(address) ~= value then space:write_u8(address, value) end
        if not announced then
            emu.print_info(string.format('LUNCHBOX_ARCADE_BLOOD_APPLIED: %s address=%x value=%d',
                enabled and 'red' or 'censored', address, value))
            announced = true
        end
    end
    apply()
    -- Also handles manual/automatic state loads and resets. Only a few byte
    -- reads each frame; no polling process, disk I/O, or startup wait.
    -- MAME reruns autoboot scripts after reset. Replace the subscription,
    -- rather than accumulating another callback on each soft reset.
    if emu.add_machine_frame_notifier then
        if _G.lunchbox_blood_subscription then _G.lunchbox_blood_subscription:unsubscribe() end
        _G.lunchbox_blood_subscription = emu.add_machine_frame_notifier(apply)
    else
        _G.lunchbox_blood_apply = apply
        if not _G.lunchbox_blood_callback_registered then
            emu.register_frame_done(function() _G.lunchbox_blood_apply() end)
            _G.lunchbox_blood_callback_registered = true
        end
    end
end

return adapter
