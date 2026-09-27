-- Run from the repository root: lua crates/lunchbox-app/tests/arcade_blood.lua
local adapter = dofile('crates/lunchbox-app/src/arcade_settings/neogeo_blood.lua')
local rom, ram, writes, callback, messages = {}, {}, {}, nil, {}
local function put(address, bytes)
    for i = 1, #bytes do rom[address+i-1] = bytes:byte(i) end
end
local function long(address, value)
    put(address, string.pack('>I4', value))
end
local function text(address, value) put(address, string.format('%-12s', value)) end
local function byte(address) return rom[address] or 0 end
local function fixture(pointer, index, reversed)
    for i = 0, 1023 do rom[pointer+i] = 0 end
    put(pointer, 'EXAMPLE GAME    ')
    put(pointer+0x10, '\255\255\255\255\3\255') -- one counter title
    text(pointer+0x20, 'LIVES')
    local cursor = pointer+0x2c
    for i = 0, index do
        rom[pointer+0x16+i] = 2
        text(cursor, i == index and 'BLOOD' or 'OTHER')
        text(cursor+12, reversed and 'ON' or 'OFF')
        text(cursor+24, reversed and 'OFF' or 'ON')
        cursor = cursor+36
    end
end

for index = 0, 9 do
    for _, reversed in ipairs({true, false}) do
        fixture(0x300, index, reversed)
        local dip = assert(adapter.find_blood(byte, 0x300))
        assert(dip.offset == 6+index)
        assert(dip.on == (reversed and 0 or 1))
        assert(dip.off == (reversed and 1 or 0))
    end
end
fixture(0x300, 1, false)
text(0x32c, 'BLOOD')
assert(adapter.find_blood(byte, 0x300) == nil, 'ambiguous duplicate option')
fixture(0x300, 0, false)
text(0x338, 'MAYBE')
assert(adapter.find_blood(byte, 0x300) == nil, 'unknown choice label')
fixture(0x300, 0, false)
rom[0x316] = 0x22
assert(adapter.find_blood(byte, 0x300) == nil, 'invalid default')

local function machine_fixture()
    rom, ram, writes, messages, callback = {}, {}, {}, {}, nil
    _G.lunchbox_blood_callback_registered = nil
    put(0x100, 'NEO-GEO\16') -- cartridge version byte is not always zero
    long(0x11a, 0x200300)
    long(0x11e, 0x200300)
    fixture(0x100300, 5, false)
    local space = {
        read_u8 = function(_, address) return ram[address] or 0 end,
        write_u8 = function(_, address, value)
            assert(address == 0x10fd8f, 'must not touch other RAM or NVRAM')
            writes[#writes+1] = address
            ram[address] = value
        end,
    }
    manager = {machine = {
        devices = {[':maincpu'] = {spaces = {program = space}}},
        memory = {regions = {[':cslot1:maincpu'] = {
            size = 0x200000, endianness = 'big', bitwidth = 16,
            read_u16 = function(_, address) return byte(address)*256+byte(address+1) end,
        }}},
        popmessage = function(_, message) messages[#messages+1] = message end,
    }}
    emu = {
        print_error = function(message) messages[#messages+1] = message end,
        print_info = function() end,
        register_frame_done = function(fn) callback = fn end,
    }
end

machine_fixture()
adapter.start(true)
assert(callback and #writes == 0, 'do not modify startup RAM test')
ram[0x10fd82], ram[0x10fdae], ram[0x10fdaf] = 0x80, 2, 1
callback()
assert(ram[0x10fd8f] == 1 and #writes == 1)
callback()
assert(#writes == 1, 'unchanged preference needs no write')
ram[0x10fd8f] = 0 -- restore an older state
callback()
assert(ram[0x10fd8f] == 1 and #writes == 2)
ram[0x10fdae], ram[0x10fd8f] = 0, 0
callback()
assert(ram[0x10fd8f] == 0, 'leave service menu and RAM tests alone')
ram[0x10fdae], ram[0x10fdaf] = 3, 2
callback()
assert(ram[0x10fd8f] == 1, 'reapply on a new game after reset')
adapter.start(false)
assert(ram[0x10fd8f] == 0, 'censored setting is reversible')

machine_fixture()
ram[0x10fd82], ram[0x10fdae], ram[0x10fdaf] = 0, 2, 1
adapter.start(true)
assert(#writes == 0 and #messages > 0, 'AES must not be changed')
machine_fixture()
rom[0x100] = 0
adapter.start(true)
assert(callback == nil and #writes == 0 and #messages > 0)
machine_fixture()
local subscriptions, removed = 0, 0
emu.add_machine_frame_notifier = function(fn)
    callback = fn
    subscriptions = subscriptions+1
    return {unsubscribe = function() removed = removed+1 end}
end
adapter.start(true)
adapter.start(true) -- MAME reruns the script after reset
assert(subscriptions == 2 and removed == 1, 'reset must replace the subscription')
ram[0x10fd82], ram[0x10fdae], ram[0x10fdaf] = 0x80, 3, 2
callback()
assert(ram[0x10fd8f] == 1)
print('arcade blood: metadata, RAM-test guard, save reload, reset, on/off and unsupported BIOS tests passed')
