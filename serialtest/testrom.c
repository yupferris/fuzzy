#include "stuff.h"
#include "font.h"

// Print

#define PRINT_ROWS 28
#define PRINT_COLS 48

static u16 *printMapPos;
static int printColumn;
static int printRow;

void printInit()
{
    printMapPos = (u16 *)BGMap(0);
    printColumn = printRow = 0;
}

void printStr(const char *str)
{
    while (1)
    {
        char c = *str++;

        if (!c)
            break;

        if (c == '\n' || printColumn == PRINT_COLS)
        {
            printMapPos -= printColumn;
            printColumn = 0;

            printRow++;
            if (printRow >= PRINT_ROWS)
            {
                // Once we're offscreen, wrap around to make sure this stays fast
                printInit();
            }
            else
            {
                printMapPos += 64;
            }
        }
        else
        {
            *printMapPos++ = c;
            printColumn++;
        }
    }
}

static const char digits[16] = { '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f' };

void printU32(const u32 value)
{
    char buf[9];
    int i;

    //printStr("0x");

    buf[8] = '\0';

    for (i = 0; i < 8; i++)
        buf[i] = digits[(value >> (4 * (7 - i))) & 0x0f];

    printStr(buf);
}

void printClear()
{
    int x, y;
    u16 *mapPtr = (u16 *)BGMap(0);

    for (y = 0; y < 64; y++)
    {
        for (x = 0; x < 64; x++)
            *(mapPtr++) = 0x0000;
    }

    printInit();
}

// Link

#define LINK_OK 0
#define LINK_ERR -1

#define LINK_CRAPSUM_INIT 0xfadebabe

static u8 receivePacketBuffer[256];
static int receivePacketLen;
static u32 receivePacketCrapsum;
static u32 sendPacketCrapsum;

void linkInit()
{
    HW_REGS[CCSR] = 0xff;
    HW_REGS[CCR] = 0x80;
}

int linkTransferByte(const u8 sendByte)
{
    const int MAX_WAIT_TRIES = 1000;
    int waitTries;

    HW_REGS[CDTR] = sendByte;

    waitTries = 0;
    HW_REGS[CCR] = 0x94;
    while (HW_REGS[CCR] & 0x02)
    {
        waitTries++;
        if (waitTries >= MAX_WAIT_TRIES)
            return LINK_ERR;
    }

    return HW_REGS[CDRR];
}

void linkUpdateCrapsum(u32 *state, u8 byte)
{
    *state = (*state << 3) | (*state >> 29) ^ byte;
}

int linkReceivePacket()
{
    const u8 HANDSHAKE = 0xaa;
    const int MAX_HANDSHAKE_TRIES = 20;
    int i;
    int handshakeTries;

    receivePacketCrapsum = LINK_CRAPSUM_INIT;

    // Pull bytes until we receive handshake byte
    handshakeTries = 0;
    while (1)
    {
        int receivedByte = linkTransferByte(0x00);
        if (receivedByte == LINK_ERR)
            return LINK_ERR;

        if (receivedByte == HANDSHAKE)
            break;

        handshakeTries++;
        if (handshakeTries >= MAX_HANDSHAKE_TRIES)
            return LINK_ERR;
    }

    // Echo handshake byte
    if (linkTransferByte(HANDSHAKE) == LINK_ERR)
        return LINK_ERR;

    // Receive packet length
    receivePacketLen = linkTransferByte(0xff);
    if (receivePacketLen == LINK_ERR)
        return;
    receivePacketLen++;

    // Receive bytes
    for (i = 0; i < receivePacketLen; i++)
    {
        int receivedByte = linkTransferByte(i);
        if (receivedByte == LINK_ERR)
            return LINK_ERR;

        receivePacketBuffer[i] = receivedByte;

        linkUpdateCrapsum(&receivePacketCrapsum, receivedByte);
    }

    return LINK_OK;
}

int linkSendPacket(const u8 *packetBuffer, int packetLen)
{
    const u8 HANDSHAKE = 0x55;
    const int MAX_HANDSHAKE_TRIES = 20;
    int i;
    int handshakeTries;

    // Check packet length, reject if not valid
    if (packetLen < 1 || packetLen > 256)
        return LINK_ERR;

    sendPacketCrapsum = LINK_CRAPSUM_INIT;

    // Pull bytes until we receive handshake byte
    handshakeTries = 0;
    while (1)
    {
        int receivedByte = linkTransferByte(0x00);
        if (receivedByte == LINK_ERR)
            return LINK_ERR;

        if (receivedByte == HANDSHAKE)
            break;

        handshakeTries++;
        if (handshakeTries >= MAX_HANDSHAKE_TRIES)
            return LINK_ERR;
    }

    // Echo handshake byte
    if (linkTransferByte(HANDSHAKE) == LINK_ERR)
        return LINK_ERR;

    // Send packet length
    if (linkTransferByte(packetLen - 1) == LINK_ERR)
        return LINK_ERR;

    // Send bytes
    for (i = 0; i < packetLen; i++)
    {
        u8 byte = packetBuffer[i];

        if (linkTransferByte(byte) == LINK_ERR)
            return LINK_ERR;

        linkUpdateCrapsum(&sendPacketCrapsum, byte);
    }

    return LINK_OK;
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

    // Set up printing & clear bg map data
    printInit();
    printClear();

    // Clean link state
    linkInit();

    // Let's goooo!
    printStr("link test yo\n");

    while (1)
    {
        // Receive packet
        if (linkReceivePacket() == LINK_ERR)
        {
            //printStr("f"); // Don't fill the screen with f's while waiting for stuff :)
            continue;
        }

        // Some work can be done here, but it's important that we send a response as soon as possible (else the exchange will time out)
        printStr("r");

        // Send packet
        //  For now we'll just send back the checksum from the received packet
        if (linkSendPacket((const u8 *)&receivePacketCrapsum, 4) == LINK_ERR)
        {
            printStr("f");
            continue;
        }

        // Exchange complete at this point, do whatev's until we're ready to do another exchange
        printStr("s.");
    }
}
