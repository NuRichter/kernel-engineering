/* c/drivers/acpi.c -- minimal ACPI RSDP/RSDT/MADT parser
   finds the IOAPIC base and CPU LAPIC addresses */

#include "../include/hal.h"
#include <stdint.h>
#include <stddef.h>

#define RSDP_SIG  "RSD PTR "
#define MADT_SIG  "APIC"

typedef struct __attribute__((packed)) {
    char     signature[8];
    uint8_t  checksum;
    char     oem_id[6];
    uint8_t  revision;
    uint32_t rsdt_addr;
    /* ACPI 2.0+ */
    uint32_t length;
    uint64_t xsdt_addr;
    uint8_t  ext_checksum;
    uint8_t  _reserved[3];
} Rsdp;

typedef struct __attribute__((packed)) {
    char     signature[4];
    uint32_t length;
    uint8_t  revision;
    uint8_t  checksum;
    char     oem_id[6];
    char     oem_table_id[8];
    uint32_t oem_revision;
    uint32_t creator_id;
    uint32_t creator_revision;
} SdtHeader;

typedef struct __attribute__((packed)) {
    SdtHeader header;
    uint32_t  lapic_addr;
    uint32_t  flags;
} Madt;

typedef struct __attribute__((packed)) {
    uint8_t  type;
    uint8_t  length;
} MadtEntryHdr;

typedef struct __attribute__((packed)) {
    MadtEntryHdr hdr;
    uint8_t  acpi_cpu_id;
    uint8_t  apic_id;
    uint32_t flags;
} MadtLapic;

typedef struct __attribute__((packed)) {
    MadtEntryHdr hdr;
    uint8_t  ioapic_id;
    uint8_t  _reserved;
    uint32_t ioapic_addr;
    uint32_t gsi_base;
} MadtIoapic;

static uint8_t acpi_checksum(const void *ptr, size_t len) {
    uint8_t sum = 0;
    const uint8_t *p = ptr;
    for (size_t i = 0; i < len; i++) sum += p[i];
    return sum;
}

static Rsdp *find_rsdp(void) {
    /* search EBDA and BIOS ROM area */
    const uintptr_t regions[][2] = {
        { 0x000E0000, 0x000FFFFF },
        { 0x00080000, 0x0009FFFF },
    };
    for (int r = 0; r < 2; r++) {
        for (uintptr_t p = regions[r][0]; p < regions[r][1]; p += 16) {
            if (__builtin_memcmp((void *)p, RSDP_SIG, 8) == 0) {
                Rsdp *rsdp = (Rsdp *)p;
                if (acpi_checksum(rsdp, 20) == 0) return rsdp;
            }
        }
    }
    return NULL;
}

/* exported to Rust via extern "C" */
uint64_t acpi_lapic_base  = 0;
uint64_t acpi_ioapic_base = 0;
uint8_t  acpi_cpu_count   = 0;

void acpi_init(void) {
    Rsdp *rsdp = find_rsdp();
    if (!rsdp) return;

    /* walk RSDT (32-bit pointers) */
    SdtHeader *rsdt = (SdtHeader *)(uintptr_t)rsdp->rsdt_addr;
    uint32_t  *ptrs = (uint32_t *)((uint8_t *)rsdt + sizeof(SdtHeader));
    size_t     cnt  = (rsdt->length - sizeof(SdtHeader)) / 4;

    for (size_t i = 0; i < cnt; i++) {
        SdtHeader *tbl = (SdtHeader *)(uintptr_t)ptrs[i];
        if (__builtin_memcmp(tbl->signature, MADT_SIG, 4) != 0) continue;

        Madt *madt = (Madt *)tbl;
        acpi_lapic_base = madt->lapic_addr;

        uint8_t *entry = (uint8_t *)madt + sizeof(Madt);
        uint8_t *end   = (uint8_t *)madt + madt->header.length;

        while (entry < end) {
            MadtEntryHdr *hdr = (MadtEntryHdr *)entry;
            switch (hdr->type) {
            case 0: {   /* Local APIC */
                MadtLapic *lapic = (MadtLapic *)entry;
                if (lapic->flags & 1) acpi_cpu_count++;
                break;
            }
            case 1: {   /* I/O APIC */
                MadtIoapic *ioa = (MadtIoapic *)entry;
                acpi_ioapic_base = ioa->ioapic_addr;
                break;
            }
            default: break;
            }
            entry += hdr->length;
        }
        return;
    }
}
