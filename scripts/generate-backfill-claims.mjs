#!/usr/bin/env node

import { createHash } from 'node:crypto';
import { basename, resolve } from 'node:path';
import { readFileSync, writeFileSync } from 'node:fs';

const ALLOCATION_RAW = 20_000_000n * 10_000_000n;
const EXPECTED_CLAIMANTS = 434;
const ADDRESS_PATTERN = /^[CG][A-Z2-7]{55}$/;

function fail(message) {
  throw new Error(message);
}

function parseCsv(contents) {
  const lines = contents.trim().split(/\r?\n/);
  const header = lines[0].split(',');
  const addressIndex = header.indexOf('owner_address');
  const ownerTypeIndex = header.indexOf('owner_type');
  const weightIndex = header.indexOf('flattened_total_cpal_raw');
  if (addressIndex < 0 || ownerTypeIndex < 0 || weightIndex < 0) {
    fail('CSV is missing owner_address, owner_type, or flattened_total_cpal_raw');
  }

  return lines.slice(1).map((line, index) => {
    const columns = line.split(',');
    if (columns.length !== header.length) {
      fail(`CSV row ${index + 2} has ${columns.length} columns; expected ${header.length}`);
    }
    const ownerAddress = columns[addressIndex];
    if (!ADDRESS_PATTERN.test(ownerAddress)) {
      fail(`CSV row ${index + 2} has an invalid owner address`);
    }
    const ownerType = columns[ownerTypeIndex];
    if (ownerType !== 'account' && ownerType !== 'contract') {
      fail(`CSV row ${index + 2} has an invalid owner type`);
    }
    if (!/^[0-9]+$/.test(columns[weightIndex])) {
      fail(`CSV row ${index + 2} has an invalid raw ownership value`);
    }
    return {
      source_row: index + 2,
      owner_address: ownerAddress,
      owner_type: ownerType,
      source_weight_raw: BigInt(columns[weightIndex]),
    };
  });
}

function allocate(rows) {
  const positive = rows.filter((row) => row.source_weight_raw > 0n);
  if (positive.length !== EXPECTED_CLAIMANTS) {
    fail(`Expected ${EXPECTED_CLAIMANTS} positive claimants; found ${positive.length}`);
  }
  if (new Set(positive.map((row) => row.owner_address)).size !== positive.length) {
    fail('Positive ownership rows contain duplicate addresses');
  }

  const totalWeight = positive.reduce((total, row) => total + row.source_weight_raw, 0n);
  const allocations = positive.map((row) => {
    const numerator = row.source_weight_raw * ALLOCATION_RAW;
    return {
      ...row,
      allocation_raw: numerator / totalWeight,
      fractional_remainder: numerator % totalWeight,
      rounding_adjustment_raw: 0n,
    };
  });
  const floorTotal = allocations.reduce((total, row) => total + row.allocation_raw, 0n);
  const undistributed = ALLOCATION_RAW - floorTotal;
  if (undistributed < 0n || undistributed >= BigInt(allocations.length)) {
    fail('Largest-remainder allocation is outside its expected bound');
  }

  const remainderOrder = [...allocations].sort((left, right) => {
    if (left.fractional_remainder !== right.fractional_remainder) {
      return left.fractional_remainder > right.fractional_remainder ? -1 : 1;
    }
    if (left.owner_address === right.owner_address) {
      return 0;
    }
    return left.owner_address < right.owner_address ? -1 : 1;
  });
  for (let index = 0; index < Number(undistributed); index += 1) {
    remainderOrder[index].allocation_raw += 1n;
    remainderOrder[index].rounding_adjustment_raw = 1n;
  }

  const allocated = allocations.reduce((total, row) => total + row.allocation_raw, 0n);
  if (allocated !== ALLOCATION_RAW || allocations.some((row) => row.allocation_raw <= 0n)) {
    fail('Generated allocations do not conserve the exact positive claim total');
  }
  return { allocations, floorTotal, totalWeight, undistributed };
}

if (process.argv.length !== 4) {
  fail('Usage: generate-backfill-claims.mjs INPUT.csv OUTPUT.json');
}

const inputPath = resolve(process.argv[2]);
const outputPath = resolve(process.argv[3]);
const source = readFileSync(inputPath);
const { allocations, floorTotal, totalWeight, undistributed } = allocate(
  parseCsv(source.toString('utf8'))
);
const manifest = {
  schema_version: 1,
  source_file: basename(inputPath),
  source_sha256: createHash('sha256').update(source).digest('hex'),
  weight_field: 'flattened_total_cpal_raw',
  allocation_method: 'proportional_floor_then_largest_remainder_address_tiebreak',
  allocation_total_raw: ALLOCATION_RAW.toString(),
  source_weight_total_raw: totalWeight.toString(),
  claimant_count: allocations.length,
  claimant_type_counts: {
    account: allocations.filter((row) => row.owner_type === 'account').length,
    contract: allocations.filter((row) => row.owner_type === 'contract').length,
  },
  floor_total_raw: floorTotal.toString(),
  largest_remainder_units: undistributed.toString(),
  allocations: allocations.map((row) => ({
    source_row: row.source_row,
    owner_address: row.owner_address,
    owner_type: row.owner_type,
    source_weight_raw: row.source_weight_raw.toString(),
    allocation_raw: row.allocation_raw.toString(),
    rounding_adjustment_raw: row.rounding_adjustment_raw.toString(),
  })),
};

writeFileSync(outputPath, `${JSON.stringify(manifest, null, 2)}\n`);
