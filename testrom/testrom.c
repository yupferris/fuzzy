#include "stuff.h"
#include "font.h"

const u32 reg_start_values[4] =
{
    0xdeadbeef,
    0xfadebabe,
    0x55555555,
    0xaaaaaaaa,
};

#define PRINT_COLS 48

static u16 *print_map_pos;
static int print_column;

void init_print()
{
    print_map_pos = (u16 *)BGMap(0);
    print_column = 0;
}

void print_str(char *str)
{
    while (1)
    {
        char c = *str++;

        if (!c)
            break;

        if (c == '\n' || print_column == PRINT_COLS)
        {
            print_map_pos -= print_column;
            print_map_pos += 64;
            print_column = 0;
        }
        else
        {
            *print_map_pos++ = c;
            print_column++;
        }
    }
}

static const char digits[16] = { '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f' };

void print_u32(u32 value)
{
    char buf[9];
    int i;

    //print_str("0x");

    buf[8] = '\0';

    for (i = 0; i < 8; i++)
        buf[i] = digits[(value >> (4 * (7 - i))) & 0x0f];

    print_str(buf);
}

void clear()
{
    int x, y;
    u8 *mapPtr = (u8 *)BGMap(0);

    for (y = 0; y < 64; y++)
    {
        for (x = 0; x < 64; x++)
        {
            u8 c = x + y;
            *mapPtr = 0x00;
            mapPtr++;
            *mapPtr = 0x00;
            mapPtr++;
        }
    }

    init_print();
}

int main()
{
    vbSetColTable();

    //display setup
    VIP_REGS[REST] = 0;
    VIP_REGS[XPCTRL] = VIP_REGS[XPSTTS] | XPEN;
    VIP_REGS[DPCTRL] = VIP_REGS[DPSTTS] | (SYNCE | RE | DISP);
    VIP_REGS[FRMCYC] = 0;
    VIP_REGS[INTCLR] = VIP_REGS[INTPND];
    while (!(VIP_REGS[DPSTTS] & 0x3C)); // Wait for VBLANK (probably)

    VIP_REGS[BRTA]  = 0;
    VIP_REGS[BRTB]  = 0;
    VIP_REGS[BRTC]  = 0;
    VIP_REGS[GPLT0] = 0xE4; /* Set all eight palettes to: 11100100 */
    VIP_REGS[GPLT1] = 0xE4; /* (i.e. "Normal" dark to light progression.) */
    VIP_REGS[GPLT2] = 0xE4;
    VIP_REGS[GPLT3] = 0xE4;
    VIP_REGS[JPLT0] = 0xE4;
    VIP_REGS[JPLT1] = 0xE4;
    VIP_REGS[JPLT2] = 0xE4;
    VIP_REGS[JPLT3] = 0xE4;
    VIP_REGS[BKCOL] = 0;    /* Clear the screen to black before rendering */
    
    SET_BRIGHT(32, 64, 32);
    
    // Set up worlds
    WA[31].head = WRLD_END;
    WA[31].head = (WRLD_LON | WRLD_RON | WRLD_OVR);
    WA[31].w = 384;
    WA[31].h = 224;

    WA[30].head = WRLD_END;
    
    // Set up char data
    {
        const unsigned int *src = FontTiles;
        unsigned int *dst = (unsigned int *)CharSeg0;
        int i;

        for (i = 0; i < 1024; i++)
        {
            *dst = *src;
            src++;
            dst++;
        }
    }
    
    // Set up display
    VIP_REGS[DPCTRL] = VIP_REGS[DPSTTS] | (SYNCE | RE | DISP);
    
    // Set up drawing
    VIP_REGS[XPCTRL] = VIP_REGS[XPSTTS] | XPEN;

    // Set up printing
    init_print();

    // Clear bg map data
    clear();

    // Print results (stored in CharSeg3)
    {
        u32 *results = (u32 *)CharSeg3;

        print_str("gpr00:");
        print_u32(*results++);
        print_str("  gpr01:");
        print_u32(*results++);
        print_str("  gpr02:");
        print_u32(*results++);
        print_str("\n");
        print_str("gpr03:");
        print_u32(*results++);
        print_str("  gpr04:");
        print_u32(*results++);
        print_str("  gpr05:");
        print_u32(*results++);
        print_str("\n");
        print_str("gpr06:");
        print_u32(*results++);
        print_str("  gpr07:");
        print_u32(*results++);
        print_str("  gpr08:");
        print_u32(*results++);
        print_str("\n");
        print_str("gpr09:");
        print_u32(*results++);
        print_str("  gpr10:");
        print_u32(*results++);
        print_str("  gpr11:");
        print_u32(*results++);
        print_str("\n");
        print_str("gpr12:");
        print_u32(*results++);
        print_str("  gpr13:");
        print_u32(*results++);
        print_str("  gpr14:");
        print_u32(*results++);
        print_str("\n");
        print_str("gpr15:");
        print_u32(*results++);
        print_str("  gpr16:");
        print_u32(*results++);
        print_str("  gpr17:");
        print_u32(*results++);
        print_str("\n");
        print_str("gpr18:");
        print_u32(*results++);
        print_str("  gpr19:");
        print_u32(*results++);
        print_str("  gpr20:");
        print_u32(*results++);
        print_str("\n");
        print_str("gpr21:");
        print_u32(*results++);
        print_str("  gpr22:");
        print_u32(*results++);
        print_str("  gpr23:");
        print_u32(*results++);
        print_str("\n");
        print_str("gpr24:");
        print_u32(*results++);
        print_str("  gpr25:");
        print_u32(*results++);
        print_str("  gpr26:");
        print_u32(*results++);
        print_str("\n");
        print_str("gpr27:");
        print_u32(*results++);
        print_str("  gpr28:");
        print_u32(*results++);
        print_str("  gpr29:");
        print_u32(*results++);
        print_str("\n");
        print_str("gpr30:");
        print_u32(*results++);
        print_str("  gpr31:");
        print_u32(*results++);
        print_str("\n");
        print_str("psw:");
        print_u32(*results++);
        print_str("\n\n");
    }

    print_str("test complete");

    while (1)
    {
        // Wheeeee
    }
}
