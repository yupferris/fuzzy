#include <avr/io.h>
#include <avr/pgmspace.h>
#include <stdint.h>
#include <util/delay.h>
#include "usb_serial.h"

#define CPU_PRESCALE(n) (CLKPR = 0x80, CLKPR = (n))
#define CPU_16MHz       0x00
#define CPU_8MHz        0x01
#define CPU_4MHz        0x02
#define CPU_2MHz        0x03
#define CPU_1MHz        0x04
#define CPU_500kHz      0x05
#define CPU_250kHz      0x06
#define CPU_125kHz      0x07
#define CPU_62kHz       0x08

// PORTB (VB)
#define CONTROL_SHIFT  (0)
#define CLOCK_SHIFT    (1)
#define DATA_IN_SHIFT  (2) // From VB
#define DATA_OUT_SHIFT (3) // To VB

#define CONTROL  (1 << CONTROL_SHIFT)
#define CLOCK    (1 << CLOCK_SHIFT)
#define DATA_IN  (1 << DATA_IN_SHIFT)
#define DATA_OUT (1 << DATA_OUT_SHIFT)

// PORTD
#define LED_SHIFT (6)

#define LED (1 << LED_SHIFT)

// String must reside in flash mem (using PSTR)
void sendStr(const char *s)
{
    while (1)
    {
        char c = pgm_read_byte(s++);
        if (!c)
            break;
        usb_serial_putchar(c);
    }
}

uint8_t transferByte(uint8_t sendByte)
{
    // Initiate transfer
    //  Bring CONTROL low and CLOCK high, set ddr
    PORTB = CLOCK;
    DDRB = CONTROL | CLOCK | DATA_OUT;

    uint8_t receivedByte = 0;

    // Transfer 8 bits
    for (int i = 0; i < 8; i++)
    {
        // Clock low
        PORTB &= ~CLOCK;
        // Set DATA_OUT to send bit
        PORTB &= ~DATA_OUT;
        PORTB |= ((sendByte >> (7 - i)) & 1) << DATA_OUT_SHIFT;
        _delay_us(5.5);
        // Clock high
        PORTB |= CLOCK;
        // Receive bit from DATA_IN
        receivedByte <<= 1;
        receivedByte |= (PINB >> DATA_IN_SHIFT) & 1;
        _delay_us(5.5);
    }

    // Release CONTROL line
    DDRB = CLOCK | DATA_OUT;

    return receivedByte;
}

static uint8_t packetBuffer[256];

int performExchange(int sendPacketLen) {
    const int MAX_HANDSHAKE_TRIES = 20;

    // Send packet
    {
        //  Send handshake byte until we receive echo back
        const uint8_t HANDSHAKE = 0xaa;
        int handshakeTries = 0;
        while (1)
        {
            uint8_t receivedByte = transferByte(HANDSHAKE);
            if (receivedByte == HANDSHAKE)
                break;

            handshakeTries++;
            if (handshakeTries >= MAX_HANDSHAKE_TRIES)
                return -1;
        }

        //  Send packet length
        transferByte(sendPacketLen - 1);

        //  Send data bytes
        for (int i = 0; i < sendPacketLen; i++)
            transferByte(packetBuffer[i]);
    }

    // Need to wait a small period before reading the receive packet in order to let the VB prepare its response
    //  Note that we can't wait too long, or our exchange will time out
    _delay_us(100.0);

    // Receive packet
    {
        const uint8_t HANDSHAKE = 0x55;
        int handshakeTries = 0;
        while (1)
        {
            uint8_t receivedByte = transferByte(HANDSHAKE);
            if (receivedByte == HANDSHAKE)
                break;

            handshakeTries++;
            if (handshakeTries >= MAX_HANDSHAKE_TRIES)
                return -1;
        }

        //  Receive length
        int receivedPacketLen = (int)transferByte(HANDSHAKE) + 1;

        //  Receive data bytes
        for (int i = 0; i < receivedPacketLen; i++)
            packetBuffer[i] = transferByte(0x00);

        return receivedPacketLen;
    }
}

int main(void)
{
    // 16mhz clock
    CPU_PRESCALE(CPU_16MHz);

    // Turn on LED
    DDRD |= LED;
    PORTD |= LED;

    // Initialize USB and wait for host config (may wait forever if powered on with no host connected)
    usb_init();
    while (!usb_configured())
        ;

    while (1)
    {
connectLoop:
        // Wait for host to set DTR
        while (!(usb_serial_get_control() & USB_SERIAL_DTR))
            ;

        // Discard any junk input/messages
        usb_serial_flush_input();

        // Send handshake
        sendStr(PSTR("HANDSHAKE YO"));

        while (1)
        {
            int16_t c;

packetLoop:
            // Read send packet len
            c = usb_serial_getchar();

            if (c == -1)
            {
                if (!usb_configured() || !(usb_serial_get_control() & USB_SERIAL_DTR))
                {
                    // Host no longer connected, reconnect
                    goto connectLoop;
                }
                else
                {
                    // Timeout, try again
                    goto packetLoop;
                }
            }

            int len = (int)c + 1;

            // Read send packet bytes
            for (int i = 0; i < len; i++)
            {
                const int MAX_BYTE_TRIES = 20;
                int byteTries = 0;

                int16_t c;

packetByteLoop:
                c = usb_serial_getchar();

                if (c == -1)
                {
                    if (!usb_configured() || !(usb_serial_get_control() & USB_SERIAL_DTR))
                    {
                        // Host no longer connected, reconnect
                        goto connectLoop;
                    }
                    else
                    {
                        // Timeout
                        byteTries++;
                        if (byteTries >= MAX_BYTE_TRIES)
                        {
                            // Too many tries; give up and move on to next exchange
                            goto packetLoop;
                        }

                        // Try again
                        goto packetByteLoop;
                    }
                }

                packetBuffer[i] = c;
            }

            // Perform exchange
            int receivedPacketLen = performExchange(len);
            if (receivedPacketLen == -1)
            {
                // Exchange failed, give up and move on to the next exchange
                goto packetLoop;
            }

            // Write received packet len
            usb_serial_putchar(receivedPacketLen - 1);

            // Write received packet bytes
            usb_serial_write(packetBuffer, receivedPacketLen);

            // Somehow flushing output with certain buffer lengths (127, 191, 255) seems to hang!
            //usb_serial_flush_output();
        }
    }
}
