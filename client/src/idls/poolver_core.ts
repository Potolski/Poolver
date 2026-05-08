/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/poolver_core.json`.
 */
export type PoolverCore = {
  "address": "2SsxJqMCYKCYesfzfXASgAPPz153j8tYMXpMKKmt2QXk",
  "metadata": {
    "name": "poolverCore",
    "version": "0.1.0",
    "spec": "0.1.0",
    "description": "Poolver V1 core program (protocol config, KYC, pools, participants)"
  },
  "docs": [
    "Poolver core program. Step-5 surface:",
    "- `initialize_protocol`",
    "- `mock_issue_kyc`            (gated under `mock-kyc` feature; spec §9.11)",
    "- `initialize_user_reputation`",
    "- `create_pool`",
    "- `join_pool`",
    "- `contribute`                (step 5)",
    "- `advance_month`             (step 5)",
    "",
    "Subsequent steps will append `commit_bid`, `reveal_bid` (step 6),",
    "`select_winner` (step 7), `claim_winning` (step 8),",
    "`liquidate_default` (step 10), `distribute_yield` (step 9),",
    "`emergency_pause` / `emergency_unpause`, and `seed_reserve` (proxy)."
  ],
  "instructions": [
    {
      "name": "adminCloseProtocol",
      "discriminator": [
        221,
        207,
        69,
        101,
        75,
        85,
        171,
        12
      ],
      "accounts": [
        {
          "name": "admin",
          "writable": true,
          "signer": true,
          "relations": [
            "protocolConfig"
          ]
        },
        {
          "name": "protocolConfig",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "protocolFeeVault",
          "docs": [
            "We don't deserialize as `TokenAccount` because the closed-state of the",
            "account post-CPI would prevent Anchor's account-info drop check from",
            "matching. The `seeds + bump` constraint is the only validation needed",
            "— any account at this PDA is, by construction, the protocol fee vault."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  102,
                  101,
                  101,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              }
            ]
          }
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        }
      ],
      "args": []
    },
    {
      "name": "adminSkipPhase",
      "discriminator": [
        67,
        117,
        70,
        195,
        210,
        64,
        92,
        180
      ],
      "accounts": [
        {
          "name": "admin",
          "signer": true
        },
        {
          "name": "protocolConfig",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "pool",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  111,
                  111,
                  108
                ]
              },
              {
                "kind": "account",
                "path": "pool.creator",
                "account": "pool"
              },
              {
                "kind": "account",
                "path": "pool.pool_id",
                "account": "pool"
              }
            ]
          }
        }
      ],
      "args": []
    },
    {
      "name": "advanceMonth",
      "discriminator": [
        221,
        78,
        214,
        206,
        213,
        3,
        109,
        206
      ],
      "accounts": [
        {
          "name": "caller",
          "docs": [
            "Anyone — permissionless. The signer just pays the tx fee."
          ],
          "signer": true
        },
        {
          "name": "protocolConfig",
          "docs": [
            "Read-only protocol config; the only thing we need from it is the",
            "`paused` flag. Box'd to match the rest of the program."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "pool",
          "writable": true
        }
      ],
      "args": []
    },
    {
      "name": "claimWinning",
      "discriminator": [
        72,
        152,
        171,
        92,
        123,
        244,
        179,
        127
      ],
      "accounts": [
        {
          "name": "winner",
          "docs": [
            "The selected winner. Signs:",
            "- the SPL transfer from their ATA into `collateral_vault`",
            "- the tx fee",
            "Authorization: `winner.key() == pool.winners[current_month-1].winner`,",
            "enforced inside the handler."
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "protocolConfig",
          "docs": [
            "Protocol config — manually deserialized (SPEC_QUESTION-15).",
            "handler."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "pool",
          "docs": [
            "The pool. Mut because we write `winners[m-1].claimed`,",
            "`bid_credit_balance`, `total_distributed`."
          ],
          "writable": true
        },
        {
          "name": "participant",
          "docs": [
            "Per-(pool, winner) participant record. PDA seed binding doubles as",
            "the \"is the caller a participant?\" check."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  97,
                  114,
                  116,
                  105,
                  99,
                  105,
                  112,
                  97,
                  110,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              },
              {
                "kind": "account",
                "path": "winner"
              }
            ]
          }
        },
        {
          "name": "userReputation",
          "docs": [
            "Winner's reputation — `total_received_lifetime` is incremented."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  114,
                  101,
                  112,
                  117,
                  116,
                  97,
                  116,
                  105,
                  111,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "winner"
              }
            ]
          }
        },
        {
          "name": "userKyc",
          "docs": [
            "Winner's KYC attestation — Full level required at claim time",
            "(defence-in-depth; `select_winner` already gated this but the",
            "attestation may have expired between selection and claim).",
            "handler via manual deserialization (SPEC_QUESTION-15)."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  107,
                  121,
                  99
                ]
              },
              {
                "kind": "account",
                "path": "winner"
              }
            ]
          }
        },
        {
          "name": "winnerUsdc",
          "docs": [
            "Winner's USDC ATA. Receives `net_payout`, sources `total_collateral_required`.",
            "token account in CPI helper."
          ],
          "writable": true
        },
        {
          "name": "poolUsdcVault",
          "docs": [
            "Pool USDC vault. PDA-owned token account; signs both the payout",
            "transfer and the protocol-fee + reserve-deposit transfers."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  111,
                  111,
                  108,
                  95,
                  117,
                  115,
                  100,
                  99,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              }
            ]
          }
        },
        {
          "name": "collateralVault",
          "docs": [
            "Collateral vault. Receives `total_collateral_required`."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  108,
                  108,
                  97,
                  116,
                  101,
                  114,
                  97,
                  108,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              }
            ]
          }
        },
        {
          "name": "protocolFeeVault",
          "docs": [
            "Protocol fee SPL vault. Receives 5% of `winning_bid`.",
            "validated in handler."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  102,
                  101,
                  101,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              }
            ]
          }
        },
        {
          "name": "coreInvoker",
          "docs": [
            "`core_invoker` PDA — co-signs reserve + yield-vault CPIs (arch §5.2)."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  114,
                  101,
                  95,
                  105,
                  110,
                  118,
                  111,
                  107,
                  101,
                  114
                ]
              }
            ]
          }
        },
        {
          "name": "reserveFund",
          "docs": [
            "re-derive in the handler against `pool.tier` for INV-4."
          ],
          "writable": true
        },
        {
          "name": "reserveUsdcVault",
          "writable": true
        },
        {
          "name": "reserveProgram",
          "address": "CfxRT3jsXWQZRev67ztqaNKCrHaKF6ieW9a1E8NDPvnx"
        },
        {
          "name": "adapterState",
          "writable": true
        },
        {
          "name": "adapterUsdcVault",
          "writable": true
        },
        {
          "name": "yieldAdapterProgram",
          "docs": [
            "`pool.tier` in the handler via `cpi_adapter_withdraw`."
          ]
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        }
      ],
      "args": [
        {
          "name": "claimMonth",
          "type": "u8"
        }
      ]
    },
    {
      "name": "commitBid",
      "discriminator": [
        149,
        237,
        198,
        113,
        53,
        66,
        70,
        76
      ],
      "accounts": [
        {
          "name": "user",
          "docs": [
            "The bidder. Pays the `bid` PDA rent and signs the stake transfer."
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "protocolConfig",
          "docs": [
            "Protocol config — read solely for the pause flag (INV-25).",
            "Manually deserialized in the handler (SPEC_QUESTION-15).",
            "inside the handler."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "pool",
          "docs": [
            "Pool. Read-only here; we don't mutate any pool field.",
            "SPEC_QUESTION-15: `Box` to keep the stack frame lean."
          ]
        },
        {
          "name": "participant",
          "docs": [
            "Per-(pool, user) participant. Verified via PDA seed binding.",
            "We rely on the seed match to enforce participation; a non-",
            "participant cannot construct a valid `Participant` PDA for this",
            "pool. The `pool` / `user` constraints below are belt-and-braces."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  97,
                  114,
                  116,
                  105,
                  99,
                  105,
                  112,
                  97,
                  110,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              },
              {
                "kind": "account",
                "path": "user"
              }
            ]
          }
        },
        {
          "name": "userKyc",
          "docs": [
            "User's KYC attestation. // MOCK_KYC: V1 attestations come from",
            "`mock_issue_kyc`; production attestations come from",
            "`issue_kyc_attestation`. Verification is identical (handled by",
            "`require_full_kyc` after manual deserialization — SPEC_QUESTION-15)."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  107,
                  121,
                  99
                ]
              },
              {
                "kind": "account",
                "path": "user"
              }
            ]
          }
        },
        {
          "name": "bid",
          "docs": [
            "Sealed-bid record for (pool, current_month, user). `init` makes",
            "double-commits impossible (INV-16)."
          ],
          "writable": true
        },
        {
          "name": "userUsdc",
          "docs": [
            "User's USDC source for the 1% anti-spam stake."
          ],
          "writable": true
        },
        {
          "name": "bidStakeVault",
          "docs": [
            "Per-pool bid-stake vault. PDA-owned token account; authority is",
            "the token account itself (its seeds sign for refunds in",
            "`reveal_bid`)."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  98,
                  105,
                  100,
                  95,
                  115,
                  116,
                  97,
                  107,
                  101,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              }
            ]
          }
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        },
        {
          "name": "rent",
          "address": "SysvarRent111111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "commitHash",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        }
      ]
    },
    {
      "name": "contribute",
      "discriminator": [
        82,
        33,
        68,
        131,
        32,
        0,
        205,
        95
      ],
      "accounts": [
        {
          "name": "user",
          "docs": [
            "The participant paying their monthly contribution. Pays the SPL",
            "transfer fee from their own USDC ATA."
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "protocolConfig",
          "docs": [
            "Protocol config. Manually deserialized in the handler",
            "(SPEC_QUESTION-15). CHECK: PDA seed binding here, owner +",
            "discriminator checked manually in the handler."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "userReputation",
          "docs": [
            "User's reputation — `total_contributed_lifetime` is incremented",
            "after CPIs succeed."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  114,
                  101,
                  112,
                  117,
                  116,
                  97,
                  116,
                  105,
                  111,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "user"
              }
            ]
          }
        },
        {
          "name": "pool",
          "docs": [
            "The pool. Box'd to keep stack pressure low (large",
            "`[MonthWinner; 12]` array; SPEC_QUESTION-15)."
          ],
          "writable": true
        },
        {
          "name": "participant",
          "docs": [
            "Per-(pool, user) participant record. The PDA seed binding here",
            "also doubles as the \"is the caller a participant?\" check —",
            "non-participants can't construct a matching seed."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  97,
                  114,
                  116,
                  105,
                  99,
                  105,
                  112,
                  97,
                  110,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              },
              {
                "kind": "account",
                "path": "user"
              }
            ]
          }
        },
        {
          "name": "userUsdc",
          "docs": [
            "User's USDC source. Like `join_pool`, kept as `UncheckedAccount`",
            "to relieve stack pressure; SPL transfer enforces ownership and",
            "balance at CPI time."
          ],
          "writable": true
        },
        {
          "name": "poolUsdcVault",
          "docs": [
            "Pool USDC vault — the PDA-owned transit account. Authority",
            "derives from its own seeds (`token::authority = pool_usdc_vault`",
            "at init time)."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  111,
                  111,
                  108,
                  95,
                  117,
                  115,
                  100,
                  99,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              }
            ]
          }
        },
        {
          "name": "collateralVault",
          "docs": [
            "Collateral vault — used for the post-win release transfer. Mut",
            "because we may move USDC out of it."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  108,
                  108,
                  97,
                  116,
                  101,
                  114,
                  97,
                  108,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              }
            ]
          }
        },
        {
          "name": "protocolFeeVault",
          "docs": [
            "Protocol fee SPL vault.",
            "with `protocol_config.protocol_fee_vault`."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  102,
                  101,
                  101,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              }
            ]
          }
        },
        {
          "name": "coreInvoker",
          "docs": [
            "`core_invoker` PDA, signs the reserve / adapter CPIs."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  114,
                  101,
                  95,
                  105,
                  110,
                  118,
                  111,
                  107,
                  101,
                  114
                ]
              }
            ]
          }
        },
        {
          "name": "reserveFund",
          "writable": true
        },
        {
          "name": "reserveUsdcVault",
          "writable": true
        },
        {
          "name": "reserveProgram",
          "address": "CfxRT3jsXWQZRev67ztqaNKCrHaKF6ieW9a1E8NDPvnx"
        },
        {
          "name": "adapterState",
          "writable": true
        },
        {
          "name": "adapterUsdcVault",
          "writable": true
        },
        {
          "name": "yieldAdapterProgram",
          "docs": [
            "`cpi_adapter_deposit` against `pool.tier`."
          ]
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        }
      ],
      "args": []
    },
    {
      "name": "createPool",
      "discriminator": [
        233,
        146,
        209,
        142,
        207,
        104,
        64,
        188
      ],
      "accounts": [
        {
          "name": "creator",
          "writable": true,
          "signer": true
        },
        {
          "name": "protocolConfig",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "creatorKyc",
          "docs": [
            "Creator's KYC attestation. Must be Light or better; verification",
            "runs through the same helper used by every KYC-gated instruction",
            "(`crate::kyc::require_light_kyc`)."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  107,
                  121,
                  99
                ]
              },
              {
                "kind": "account",
                "path": "creator"
              }
            ]
          }
        },
        {
          "name": "creatorReputation",
          "docs": [
            "Creator's reputation. Existence is required so `pools_completed`",
            "can be snapshotted; we do NOT mutate it here (snapshot is taken",
            "in `join_pool` for whichever user joins)."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  114,
                  101,
                  112,
                  117,
                  116,
                  97,
                  116,
                  105,
                  111,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "creator"
              }
            ]
          }
        },
        {
          "name": "pool",
          "docs": [
            "The pool being created. Box'd to keep the stack frame lean."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  111,
                  111,
                  108
                ]
              },
              {
                "kind": "account",
                "path": "creator"
              },
              {
                "kind": "arg",
                "path": "poolId"
              }
            ]
          }
        },
        {
          "name": "usdcMint"
        },
        {
          "name": "poolUsdcVault",
          "docs": [
            "PDA-owned USDC vault for this pool's contributions. Authority is",
            "the token account itself (its seeds sign for transfers in / out)."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  111,
                  111,
                  108,
                  95,
                  117,
                  115,
                  100,
                  99,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              }
            ]
          }
        },
        {
          "name": "collateralVault",
          "docs": [
            "PDA-owned collateral vault. Same self-authority pattern."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  108,
                  108,
                  97,
                  116,
                  101,
                  114,
                  97,
                  108,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              }
            ]
          }
        },
        {
          "name": "bidStakeVault",
          "docs": [
            "PDA-owned vault that escrows the 1% anti-spam bid stakes",
            "(SPEC_QUESTION-3). Step 6 — `commit_bid` deposits, `reveal_bid`",
            "refunds, step 7's `select_winner` (or its no-reveal cleanup ix)",
            "sweeps any unrevealed stakes to the tier reserve. Self-authority",
            "pattern matches `pool_usdc_vault` and `collateral_vault`."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  98,
                  105,
                  100,
                  95,
                  115,
                  116,
                  97,
                  107,
                  101,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              }
            ]
          }
        },
        {
          "name": "coreInvoker",
          "docs": [
            "`core_invoker` PDA — used as signer for the CPI into yield-vault.",
            "into `invoke_signed`. Seeds are verified inside the adapter."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  114,
                  101,
                  95,
                  105,
                  110,
                  118,
                  111,
                  107,
                  101,
                  114
                ]
              }
            ]
          }
        },
        {
          "name": "adapterState",
          "writable": true
        },
        {
          "name": "adapterUsdcVault",
          "writable": true
        },
        {
          "name": "yieldAdapterProgram",
          "docs": [
            "via `cpi_adapter_initialize` against `pool.tier`. The hardcoded",
            "`address = poolver_yield_vault::ID` constraint that pre-step-13",
            "builds carried was dropped so Tier 1 callers can pass",
            "`poolver_yield_defi::ID`."
          ]
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        },
        {
          "name": "rent",
          "address": "SysvarRent111111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "poolId",
          "type": "u64"
        },
        {
          "name": "tier",
          "type": {
            "defined": {
              "name": "tier"
            }
          }
        },
        {
          "name": "contributionAmount",
          "type": "u64"
        },
        {
          "name": "monthDurationSeconds",
          "type": {
            "option": "i64"
          }
        }
      ]
    },
    {
      "name": "distributeYield",
      "discriminator": [
        233,
        92,
        186,
        157,
        235,
        238,
        212,
        114
      ],
      "accounts": [
        {
          "name": "caller",
          "docs": [
            "Permissionless caller — anyone can pay tx fees and trigger a",
            "harvest. In production this is a keeper bot; nothing special is",
            "required of the signer beyond rent-paying ability."
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "protocolConfig",
          "docs": [
            "Protocol config — manually deserialized. CHECK: PDA seed binding",
            "here, owner+discriminator validated in handler."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "pool",
          "docs": [
            "The pool. Mut because we write `total_yield_distributed` and",
            "`bid_credit_balance` on positive-yield paths."
          ],
          "writable": true
        },
        {
          "name": "poolUsdcVault",
          "docs": [
            "Pool USDC vault. PDA-owned token account; signs the protocol-fee",
            "transfer and the reserve-deposit CPI."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  111,
                  111,
                  108,
                  95,
                  117,
                  115,
                  100,
                  99,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              }
            ]
          }
        },
        {
          "name": "protocolFeeVault",
          "docs": [
            "Protocol fee SPL vault. Receives 10% of `yield_amount`.",
            "validated in handler after manual deser."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  102,
                  101,
                  101,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              }
            ]
          }
        },
        {
          "name": "coreInvoker",
          "docs": [
            "`core_invoker` PDA — co-signs reserve + yield-vault CPIs (arch §5.2)."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  114,
                  101,
                  95,
                  105,
                  110,
                  118,
                  111,
                  107,
                  101,
                  114
                ]
              }
            ]
          }
        },
        {
          "name": "reserveFund",
          "docs": [
            "re-derive in the handler against `pool.tier` for INV-4 (tier",
            "isolation — a Tier 0 pool MUST NOT distribute yield into the",
            "Tier 1 reserve)."
          ],
          "writable": true
        },
        {
          "name": "reserveUsdcVault",
          "writable": true
        },
        {
          "name": "reserveProgram",
          "address": "CfxRT3jsXWQZRev67ztqaNKCrHaKF6ieW9a1E8NDPvnx"
        },
        {
          "name": "adapterState",
          "docs": [
            "via PDA seeds + handler-side tier-aware re-derivation."
          ],
          "writable": true
        },
        {
          "name": "adapterUsdcVault",
          "docs": [
            "via PDA seeds + handler-side tier-aware re-derivation."
          ],
          "writable": true
        },
        {
          "name": "yieldAdapterProgram",
          "docs": [
            "`pool.tier` in the handler via `cpi_adapter_harvest` /",
            "`cpi_adapter_withdraw`."
          ]
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        }
      ],
      "args": []
    },
    {
      "name": "initializeProtocol",
      "discriminator": [
        188,
        233,
        252,
        106,
        134,
        146,
        202,
        91
      ],
      "accounts": [
        {
          "name": "admin",
          "writable": true,
          "signer": true
        },
        {
          "name": "protocolConfig",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "usdcMint"
        },
        {
          "name": "protocolFeeVault",
          "docs": [
            "PDA-owned USDC token account that receives every protocol fee.",
            "The token-account itself is its own authority (its seeds sign",
            "for any future protocol-fee withdrawal — those instructions are",
            "not in scope for step 4)."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  102,
                  101,
                  101,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              }
            ]
          }
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        },
        {
          "name": "rent",
          "address": "SysvarRent111111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "initializeUserReputation",
      "discriminator": [
        58,
        224,
        89,
        218,
        243,
        208,
        126,
        131
      ],
      "accounts": [
        {
          "name": "user",
          "writable": true,
          "signer": true
        },
        {
          "name": "reputation",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  114,
                  101,
                  112,
                  117,
                  116,
                  97,
                  116,
                  105,
                  111,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "user"
              }
            ]
          }
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "joinPool",
      "discriminator": [
        14,
        65,
        62,
        16,
        116,
        17,
        195,
        107
      ],
      "accounts": [
        {
          "name": "user",
          "docs": [
            "Joining user — pays for the Participant PDA rent and signs the",
            "initial USDC transfer into the pool vault."
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "protocolConfig",
          "docs": [
            "Protocol config. Manually deserialized inside the handler to",
            "keep `try_accounts`'s stack frame within the 4 KB BPF budget",
            "(SPEC_QUESTION-15). Anchor's `Account<'info, ProtocolConfig>`",
            "would normally enforce ownership + discriminator + bump in",
            "`try_accounts`; here we re-do those checks manually inside the",
            "handler (still in poolver-core's frame, so no security loss).",
            "owner check + discriminator check."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "userKyc",
          "docs": [
            "User's KYC attestation. // MOCK_KYC: V1 attestations come from",
            "`mock_issue_kyc`; production attestations come from",
            "`issue_kyc_attestation`. Verification is identical (handled by",
            "`require_light_kyc` after manual deserialization)."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  107,
                  121,
                  99
                ]
              },
              {
                "kind": "account",
                "path": "user"
              }
            ]
          }
        },
        {
          "name": "userReputation",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  114,
                  101,
                  112,
                  117,
                  116,
                  97,
                  116,
                  105,
                  111,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "user"
              }
            ]
          }
        },
        {
          "name": "pool",
          "docs": [
            "Pool. Box'd to keep stack pressure low (SPEC_QUESTION-15). The",
            "large `[Option<MonthWinner>; 12]` array (1200 bytes) lives on",
            "the heap; only the box pointer occupies the JoinPool stack",
            "frame."
          ],
          "writable": true
        },
        {
          "name": "participant",
          "docs": [
            "Per-(pool, user) participant record. Created here."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  97,
                  114,
                  116,
                  105,
                  99,
                  105,
                  112,
                  97,
                  110,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              },
              {
                "kind": "account",
                "path": "user"
              }
            ]
          }
        },
        {
          "name": "userUsdc",
          "docs": [
            "User's source USDC account. Validated as `AccountInfo` to keep",
            "the JoinPool struct under the 4 KB BPF stack budget",
            "(SPEC_QUESTION-15) — TokenAccount deserialization happens inside",
            "the SPL transfer CPI, which is the canonical authority for token",
            "account semantics anyway."
          ],
          "writable": true
        },
        {
          "name": "poolUsdcVault",
          "docs": [
            "Pool USDC vault — owned by its own PDA. Bump comes from the",
            "`seeds` clause and is required to sign downstream PDA transfers.",
            "enforce identity."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  111,
                  111,
                  108,
                  95,
                  117,
                  115,
                  100,
                  99,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              }
            ]
          }
        },
        {
          "name": "collateralVault",
          "docs": [
            "Collateral vault — receives the join collateral (1× contribution",
            "per spec §4 demo extension). Mutable since join now transfers",
            "USDC into it.",
            "enforce identity."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  108,
                  108,
                  97,
                  116,
                  101,
                  114,
                  97,
                  108,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              }
            ]
          }
        },
        {
          "name": "protocolFeeVault",
          "docs": [
            "Protocol fee SPL vault.",
            "`protocol_fee_vault` PDA derived from `[PROTOCOL_FEE_VAULT_SEED]`.",
            "The handler also verifies `protocol_config.protocol_fee_vault`",
            "equality after manual deserialization."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  102,
                  101,
                  101,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              }
            ]
          }
        },
        {
          "name": "coreInvoker",
          "docs": [
            "`core_invoker` PDA, signs the reserve / adapter CPIs."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  114,
                  101,
                  95,
                  105,
                  110,
                  118,
                  111,
                  107,
                  101,
                  114
                ]
              }
            ]
          }
        },
        {
          "name": "reserveFund",
          "writable": true
        },
        {
          "name": "reserveUsdcVault",
          "writable": true
        },
        {
          "name": "reserveProgram",
          "address": "CfxRT3jsXWQZRev67ztqaNKCrHaKF6ieW9a1E8NDPvnx"
        },
        {
          "name": "adapterState",
          "writable": true
        },
        {
          "name": "adapterUsdcVault",
          "writable": true
        },
        {
          "name": "yieldAdapterProgram",
          "docs": [
            "`pool.tier` in the handler via `cpi_adapter_deposit`. The",
            "hardcoded `address = poolver_yield_vault::ID` constraint that",
            "pre-step-13 builds carried was dropped so Tier 1 join calls can",
            "pass `poolver_yield_defi::ID`."
          ]
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        },
        {
          "name": "rent",
          "address": "SysvarRent111111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "liquidateDefault",
      "discriminator": [
        165,
        16,
        163,
        173,
        215,
        241,
        240,
        28
      ],
      "accounts": [
        {
          "name": "caller",
          "docs": [
            "Permissionless. Pays the tx fee."
          ],
          "signer": true
        },
        {
          "name": "protocolConfig",
          "docs": [
            "Protocol config — read-only (pause check)."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "pool",
          "docs": [
            "The pool. Mut because we update `total_collateral_locked` on",
            "Case A (collateral leaves the protocol's collateral vault and",
            "rotates into pool_usdc_vault)."
          ],
          "writable": true
        },
        {
          "name": "participant",
          "docs": [
            "The defaulting participant."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  97,
                  114,
                  116,
                  105,
                  99,
                  105,
                  112,
                  97,
                  110,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              },
              {
                "kind": "account",
                "path": "participant.user",
                "account": "participant"
              }
            ]
          }
        },
        {
          "name": "userReputation",
          "docs": [
            "Defaulter's reputation — `pools_defaulted` is incremented.",
            "SPEC_QUESTION-11: this is the global gate that future",
            "`join_pool` calls check; defaulting in pool A blocks new joins",
            "across the board but does NOT yank the user from other active",
            "pools."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  114,
                  101,
                  112,
                  117,
                  116,
                  97,
                  116,
                  105,
                  111,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "participant.user",
                "account": "participant"
              }
            ]
          }
        },
        {
          "name": "poolUsdcVault",
          "docs": [
            "Pool USDC vault — receives the liquidated collateral + reserve",
            "drawdown so the remaining-months contributions are pre-funded."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  111,
                  111,
                  108,
                  95,
                  117,
                  115,
                  100,
                  99,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              }
            ]
          }
        },
        {
          "name": "collateralVault",
          "docs": [
            "Collateral vault — drained on Case A."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  108,
                  108,
                  97,
                  116,
                  101,
                  114,
                  97,
                  108,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              }
            ]
          }
        },
        {
          "name": "coreInvoker",
          "docs": [
            "`core_invoker` PDA — co-signs the reserve `draw` CPI (arch §5.2)."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  114,
                  101,
                  95,
                  105,
                  110,
                  118,
                  111,
                  107,
                  101,
                  114
                ]
              }
            ]
          }
        },
        {
          "name": "reserveFund",
          "docs": [
            "reserve program also enforces its own seeds."
          ],
          "writable": true
        },
        {
          "name": "reserveUsdcVault",
          "writable": true
        },
        {
          "name": "reserveProgram",
          "address": "CfxRT3jsXWQZRev67ztqaNKCrHaKF6ieW9a1E8NDPvnx"
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        }
      ],
      "args": []
    },
    {
      "name": "markLatePayment",
      "discriminator": [
        236,
        51,
        25,
        113,
        115,
        163,
        122,
        74
      ],
      "accounts": [
        {
          "name": "caller",
          "docs": [
            "Permissionless keeper. Pays the tx fee only."
          ],
          "signer": true
        },
        {
          "name": "protocolConfig",
          "docs": [
            "Read-only protocol config — pause check (INV-25)."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "pool",
          "docs": [
            "Pool. Read-only — defaults touch only Participant state."
          ]
        },
        {
          "name": "participant",
          "docs": [
            "The participant being marked late."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  97,
                  114,
                  116,
                  105,
                  99,
                  105,
                  112,
                  97,
                  110,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              },
              {
                "kind": "account",
                "path": "participant.user",
                "account": "participant"
              }
            ]
          }
        }
      ],
      "args": []
    },
    {
      "name": "mockIssueKyc",
      "discriminator": [
        151,
        73,
        154,
        96,
        171,
        181,
        100,
        60
      ],
      "accounts": [
        {
          "name": "admin",
          "docs": [
            "Pays for and signs the attestation issuance. In V1 the admin and",
            "kyc_oracle are the same key (set by `initialize_protocol`), and",
            "the constraint below pins this to `protocol_config.kyc_oracle`.",
            "SPEC_QUESTION-26: production rotates kyc_oracle to a dedicated",
            "HSM-backed key; this signer constraint stays unchanged because",
            "the verifier reads kyc_oracle from on-chain config."
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "protocolConfig",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "userPubkey",
          "docs": [
            "pubkey on the attestation. Anchor cannot type this as a SystemAccount",
            "because the user account need not exist on chain yet."
          ]
        },
        {
          "name": "attestation",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  107,
                  121,
                  99
                ]
              },
              {
                "kind": "account",
                "path": "userPubkey"
              }
            ]
          }
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "user",
          "type": "pubkey"
        },
        {
          "name": "level",
          "type": {
            "defined": {
              "name": "kycLevel"
            }
          }
        }
      ]
    },
    {
      "name": "refundCollateral",
      "docs": [
        "Refund a non-defaulting participant's locked collateral after",
        "the pool has completed. Permissionless — anyone may call."
      ],
      "discriminator": [
        200,
        219,
        212,
        225,
        216,
        188,
        155,
        225
      ],
      "accounts": [
        {
          "name": "caller",
          "docs": [
            "Permissionless caller — pays the tx fee. The refund still goes",
            "to `participant.user`'s ATA, not the caller."
          ],
          "signer": true
        },
        {
          "name": "pool",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  111,
                  111,
                  108
                ]
              },
              {
                "kind": "account",
                "path": "pool.creator",
                "account": "pool"
              },
              {
                "kind": "account",
                "path": "pool.pool_id",
                "account": "pool"
              }
            ]
          }
        },
        {
          "name": "participant",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  97,
                  114,
                  116,
                  105,
                  99,
                  105,
                  112,
                  97,
                  110,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              },
              {
                "kind": "account",
                "path": "participant.user",
                "account": "participant"
              }
            ]
          }
        },
        {
          "name": "participantUsdc",
          "writable": true
        },
        {
          "name": "collateralVault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  108,
                  108,
                  97,
                  116,
                  101,
                  114,
                  97,
                  108,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              }
            ]
          }
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        }
      ],
      "args": []
    },
    {
      "name": "revealBid",
      "discriminator": [
        48,
        73,
        28,
        255,
        202,
        126,
        236,
        196
      ],
      "accounts": [
        {
          "name": "user",
          "docs": [
            "The bidder. The `user` field is verified against the seed-bound",
            "`bid.user` so spoofing is structurally impossible — Anchor's PDA",
            "derivation requires the bid PDA seeds to include this signer."
          ],
          "signer": true
        },
        {
          "name": "protocolConfig",
          "docs": [
            "Protocol config. Manually deserialized (SPEC_QUESTION-15).",
            "inside the handler."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "pool",
          "docs": [
            "Pool. Read-only; we only read tier + windows + contribution."
          ]
        },
        {
          "name": "participant",
          "docs": [
            "Per-(pool, user) participant. Read-only — we only re-check",
            "`has_won`, `is_defaulted`, `is_suspended`."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  97,
                  114,
                  116,
                  105,
                  99,
                  105,
                  112,
                  97,
                  110,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              },
              {
                "kind": "account",
                "path": "user"
              }
            ]
          }
        },
        {
          "name": "bid",
          "docs": [
            "Sealed-bid record. The PDA seed includes `month`, so a `Bid` PDA",
            "for a stale month would not match this derivation. We also check",
            "`bid.month == pool.current_month` in the handler for a clearer",
            "error surface."
          ],
          "writable": true
        },
        {
          "name": "userUsdc",
          "docs": [
            "User's USDC ATA — receives the stake refund."
          ],
          "writable": true
        },
        {
          "name": "bidStakeVault",
          "docs": [
            "Per-pool bid-stake vault.",
            "withdraw the refund from it."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  98,
                  105,
                  100,
                  95,
                  115,
                  116,
                  97,
                  107,
                  101,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              }
            ]
          }
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        }
      ],
      "args": [
        {
          "name": "bidAmount",
          "type": "u64"
        },
        {
          "name": "nonce",
          "type": {
            "array": [
              "u8",
              16
            ]
          }
        }
      ]
    },
    {
      "name": "selectWinner",
      "discriminator": [
        119,
        66,
        44,
        236,
        79,
        158,
        82,
        51
      ],
      "accounts": [
        {
          "name": "caller",
          "docs": [
            "Permissionless caller. Pays the tx fee. Not validated against any",
            "on-chain state — anyone is allowed to advance pool state by spec."
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "protocolConfig",
          "docs": [
            "Protocol config. Manually deserialized in the handler",
            "(SPEC_QUESTION-15). CHECK: PDA seed binding here, owner +",
            "discriminator validated inside the handler."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "pool",
          "docs": [
            "The pool. Mut because we write the winner slot."
          ],
          "writable": true
        },
        {
          "name": "bidStakeVault",
          "docs": [
            "Per-pool bid-stake vault. PDA-owned token account; signs the",
            "transfer to reserve. CHECK: PDA seed binding ensures identity."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  98,
                  105,
                  100,
                  95,
                  115,
                  116,
                  97,
                  107,
                  101,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              }
            ]
          }
        },
        {
          "name": "coreInvoker",
          "docs": [
            "`core_invoker` PDA — co-signs the reserve CPI per arch §5.2."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  114,
                  101,
                  95,
                  105,
                  110,
                  118,
                  111,
                  107,
                  101,
                  114
                ]
              }
            ]
          }
        },
        {
          "name": "reserveFund",
          "docs": [
            "Mut because deposit increments balance + inflows."
          ],
          "writable": true
        },
        {
          "name": "reserveUsdcVault",
          "docs": [
            "deposit transfers tokens in."
          ],
          "writable": true
        },
        {
          "name": "reserveProgram",
          "address": "CfxRT3jsXWQZRev67ztqaNKCrHaKF6ieW9a1E8NDPvnx"
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        }
      ],
      "args": []
    },
    {
      "name": "slashUnpaid",
      "docs": [
        "Permissionless. Slash the contribution_amount from a participant's",
        "collateral when they failed to pay this month (callable as soon",
        "as the month duration has elapsed). Forwards the slashed amount",
        "into the yield adapter so the pot stays whole."
      ],
      "discriminator": [
        157,
        6,
        181,
        187,
        102,
        173,
        238,
        79
      ],
      "accounts": [
        {
          "name": "caller",
          "docs": [
            "Permissionless caller — pays tx fee."
          ],
          "signer": true
        },
        {
          "name": "protocolConfig",
          "docs": [
            "Read-only — paused gate."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "pool",
          "writable": true
        },
        {
          "name": "participant",
          "docs": [
            "The participant being slashed."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  97,
                  114,
                  116,
                  105,
                  99,
                  105,
                  112,
                  97,
                  110,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              },
              {
                "kind": "account",
                "path": "participant.user",
                "account": "participant"
              }
            ]
          }
        },
        {
          "name": "userReputation",
          "docs": [
            "The participant's reputation account — bumped on slash."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  114,
                  101,
                  112,
                  117,
                  116,
                  97,
                  116,
                  105,
                  111,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "participant.user",
                "account": "participant"
              }
            ]
          }
        },
        {
          "name": "collateralVault",
          "docs": [
            "Collateral vault — source of the slashed funds."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  108,
                  108,
                  97,
                  116,
                  101,
                  114,
                  97,
                  108,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              }
            ]
          }
        },
        {
          "name": "poolUsdcVault",
          "docs": [
            "Pool USDC vault — transit account."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  111,
                  111,
                  108,
                  95,
                  117,
                  115,
                  100,
                  99,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              }
            ]
          }
        },
        {
          "name": "coreInvoker",
          "docs": [
            "the adapter CPI alongside `pool_usdc_vault`."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  114,
                  101,
                  95,
                  105,
                  110,
                  118,
                  111,
                  107,
                  101,
                  114
                ]
              }
            ]
          }
        },
        {
          "name": "adapterState",
          "writable": true
        },
        {
          "name": "adapterUsdcVault",
          "writable": true
        },
        {
          "name": "yieldAdapterProgram"
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        }
      ],
      "args": []
    },
    {
      "name": "suspendParticipant",
      "discriminator": [
        105,
        222,
        29,
        90,
        253,
        171,
        41,
        236
      ],
      "accounts": [
        {
          "name": "caller",
          "signer": true
        },
        {
          "name": "protocolConfig",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "pool"
        },
        {
          "name": "participant",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  97,
                  114,
                  116,
                  105,
                  99,
                  105,
                  112,
                  97,
                  110,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "pool"
              },
              {
                "kind": "account",
                "path": "participant.user",
                "account": "participant"
              }
            ]
          }
        }
      ],
      "args": []
    }
  ],
  "accounts": [
    {
      "name": "bid",
      "discriminator": [
        143,
        246,
        48,
        245,
        42,
        145,
        180,
        88
      ]
    },
    {
      "name": "kycAttestation",
      "discriminator": [
        114,
        140,
        31,
        243,
        17,
        104,
        193,
        72
      ]
    },
    {
      "name": "participant",
      "discriminator": [
        32,
        142,
        108,
        79,
        247,
        179,
        54,
        6
      ]
    },
    {
      "name": "pool",
      "discriminator": [
        241,
        154,
        109,
        4,
        17,
        177,
        109,
        188
      ]
    },
    {
      "name": "protocolConfig",
      "discriminator": [
        207,
        91,
        250,
        28,
        152,
        179,
        215,
        209
      ]
    },
    {
      "name": "userReputation",
      "discriminator": [
        86,
        95,
        94,
        218,
        215,
        219,
        207,
        37
      ]
    }
  ],
  "events": [
    {
      "name": "bidCommitted",
      "discriminator": [
        81,
        13,
        193,
        139,
        0,
        168,
        82,
        55
      ]
    },
    {
      "name": "bidDistributed",
      "discriminator": [
        176,
        34,
        205,
        113,
        210,
        179,
        62,
        132
      ]
    },
    {
      "name": "bidRevealed",
      "discriminator": [
        227,
        144,
        125,
        229,
        28,
        109,
        18,
        209
      ]
    },
    {
      "name": "bidStakeForfeited",
      "discriminator": [
        232,
        15,
        36,
        22,
        223,
        184,
        20,
        77
      ]
    },
    {
      "name": "collateralRefunded",
      "discriminator": [
        61,
        61,
        254,
        24,
        36,
        237,
        169,
        51
      ]
    },
    {
      "name": "contribution",
      "discriminator": [
        68,
        104,
        138,
        71,
        180,
        88,
        183,
        210
      ]
    },
    {
      "name": "defaultLiquidated",
      "discriminator": [
        46,
        183,
        23,
        140,
        5,
        112,
        45,
        41
      ]
    },
    {
      "name": "kycAttestationIssued",
      "discriminator": [
        122,
        247,
        41,
        229,
        169,
        40,
        166,
        128
      ]
    },
    {
      "name": "latePayment",
      "discriminator": [
        84,
        237,
        140,
        233,
        168,
        221,
        224,
        93
      ]
    },
    {
      "name": "liquidationShortfall",
      "discriminator": [
        233,
        160,
        106,
        58,
        95,
        164,
        12,
        178
      ]
    },
    {
      "name": "monthAdvanced",
      "discriminator": [
        26,
        181,
        98,
        156,
        194,
        212,
        228,
        96
      ]
    },
    {
      "name": "participantJoined",
      "discriminator": [
        48,
        182,
        206,
        15,
        56,
        181,
        24,
        253
      ]
    },
    {
      "name": "participantSlashed",
      "discriminator": [
        165,
        133,
        158,
        255,
        205,
        241,
        126,
        67
      ]
    },
    {
      "name": "participantSuspended",
      "discriminator": [
        238,
        228,
        225,
        143,
        172,
        45,
        37,
        171
      ]
    },
    {
      "name": "phaseSkipped",
      "discriminator": [
        84,
        238,
        24,
        165,
        222,
        212,
        37,
        246
      ]
    },
    {
      "name": "poolCompleted",
      "discriminator": [
        99,
        219,
        56,
        251,
        124,
        121,
        207,
        44
      ]
    },
    {
      "name": "poolCreated",
      "discriminator": [
        202,
        44,
        41,
        88,
        104,
        220,
        157,
        82
      ]
    },
    {
      "name": "poolStarted",
      "discriminator": [
        102,
        80,
        41,
        91,
        171,
        149,
        241,
        196
      ]
    },
    {
      "name": "protocolClosed",
      "discriminator": [
        71,
        219,
        226,
        173,
        20,
        219,
        33,
        137
      ]
    },
    {
      "name": "protocolInitialized",
      "discriminator": [
        173,
        122,
        168,
        254,
        9,
        118,
        76,
        132
      ]
    },
    {
      "name": "userReputationInitialized",
      "discriminator": [
        161,
        146,
        71,
        144,
        43,
        189,
        72,
        156
      ]
    },
    {
      "name": "winnerSelected",
      "discriminator": [
        245,
        110,
        152,
        173,
        193,
        48,
        133,
        5
      ]
    },
    {
      "name": "winningClaimed",
      "discriminator": [
        189,
        190,
        71,
        74,
        255,
        201,
        40,
        133
      ]
    },
    {
      "name": "yieldDistributed",
      "discriminator": [
        107,
        100,
        4,
        71,
        95,
        176,
        248,
        94
      ]
    },
    {
      "name": "yieldHarvested",
      "discriminator": [
        49,
        197,
        226,
        232,
        154,
        211,
        249,
        222
      ]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "unauthorized",
      "msg": "Caller is not authorized for this instruction"
    },
    {
      "code": 6001,
      "name": "protocolPaused",
      "msg": "Protocol is paused"
    },
    {
      "code": 6002,
      "name": "mathOverflow",
      "msg": "Arithmetic overflow"
    },
    {
      "code": 6003,
      "name": "invalidAmount",
      "msg": "Amount must be non-zero"
    },
    {
      "code": 6004,
      "name": "invalidContributionAmount",
      "msg": "Contribution amount outside the [100, 10_000] USDC range"
    },
    {
      "code": 6005,
      "name": "poolFull",
      "msg": "Pool has 12 participants and cannot accept more"
    },
    {
      "code": 6006,
      "name": "poolAlreadyStarted",
      "msg": "Pool has already started; new joins are no longer accepted"
    },
    {
      "code": 6007,
      "name": "poolComplete",
      "msg": "Pool is complete; no further mutations allowed"
    },
    {
      "code": 6008,
      "name": "tierNotYetSupported",
      "msg": "This tier is not yet supported in V1; only Vault (Tier 0) is enabled"
    },
    {
      "code": 6009,
      "name": "alreadyParticipant",
      "msg": "User is already a participant in this pool"
    },
    {
      "code": 6010,
      "name": "kycMissing",
      "msg": "User has no KYC attestation; Light KYC required to join a pool"
    },
    {
      "code": 6011,
      "name": "kycExpired",
      "msg": "KYC attestation has expired"
    },
    {
      "code": 6012,
      "name": "kycInsufficientLevel",
      "msg": "KYC attestation level is below the required threshold"
    },
    {
      "code": 6013,
      "name": "kycSanctionsHit",
      "msg": "KYC attestation flags a sanctions hit; user is blocked"
    },
    {
      "code": 6014,
      "name": "userReputationMissing",
      "msg": "UserReputation account is missing; call initialize_user_reputation first"
    },
    {
      "code": 6015,
      "name": "poolNotStarted",
      "msg": "Pool has not started; current_month must be in 1..=12"
    },
    {
      "code": 6016,
      "name": "notAParticipant",
      "msg": "Caller is not a participant of this pool"
    },
    {
      "code": 6017,
      "name": "contributionAlreadyMade",
      "msg": "Participant has already contributed for the current month"
    },
    {
      "code": 6018,
      "name": "outsideMonthWindow",
      "msg": "Outside the current-month contribution window (grace period not yet implemented)"
    },
    {
      "code": 6019,
      "name": "defaulted",
      "msg": "Participant is defaulted; contributions blocked"
    },
    {
      "code": 6020,
      "name": "suspended",
      "msg": "Participant is suspended; contributions blocked"
    },
    {
      "code": 6021,
      "name": "monthDurationNotElapsed",
      "msg": "Current month duration has not elapsed; advance_month rejected"
    },
    {
      "code": 6022,
      "name": "bidWindowClosed",
      "msg": "Bid window is closed; commits not accepted (and reveal expired)"
    },
    {
      "code": 6023,
      "name": "bidWindowOpen",
      "msg": "Bid (commit) window is still open; reveal not yet allowed"
    },
    {
      "code": 6024,
      "name": "bidExceedsCap",
      "msg": "Bid amount exceeds the per-month bid cap (20% of monthly pot)"
    },
    {
      "code": 6025,
      "name": "bidRevealMismatch",
      "msg": "Reveal hash does not match the stored commit_hash"
    },
    {
      "code": 6026,
      "name": "alreadyRevealed",
      "msg": "Bid has already been revealed; second reveal rejected"
    },
    {
      "code": 6027,
      "name": "alreadyWon",
      "msg": "Caller has already won a previous month and cannot bid again"
    },
    {
      "code": 6028,
      "name": "winnerAlreadySelected",
      "msg": "Winner has already been selected for the current month"
    },
    {
      "code": 6029,
      "name": "noEligibleParticipants",
      "msg": "No eligible participants for the lottery (all have won, defaulted, or lack Full KYC)"
    },
    {
      "code": 6030,
      "name": "selectWinnerAccountsMalformed",
      "msg": "`select_winner` `remaining_accounts` is malformed: expected (bid|participant) chunks"
    },
    {
      "code": 6031,
      "name": "winnerNotSelected",
      "msg": "Cannot advance to the next month before drawing the current month's winner"
    },
    {
      "code": 6032,
      "name": "notWinner",
      "msg": "Caller is not the selected winner for the current month"
    },
    {
      "code": 6033,
      "name": "alreadyClaimed",
      "msg": "Winner has already claimed for this month"
    },
    {
      "code": 6034,
      "name": "collateralInsufficient",
      "msg": "Winner does not have enough USDC to post the required collateral"
    },
    {
      "code": 6035,
      "name": "gracePeriodNotElapsed",
      "msg": "Grace period has not elapsed yet (mark_late) or suspension threshold not reached"
    },
    {
      "code": 6036,
      "name": "gracePeriodElapsed",
      "msg": "Grace period has elapsed; mark_late no longer accepted — call suspend_participant"
    },
    {
      "code": 6037,
      "name": "defaultThresholdNotReached",
      "msg": "30-day default threshold has not been reached; liquidation rejected"
    },
    {
      "code": 6038,
      "name": "alreadyLiquidated",
      "msg": "Participant has already been liquidated; double-liquidate rejected"
    },
    {
      "code": 6039,
      "name": "alreadyMarkedLate",
      "msg": "Participant has already been marked late this month"
    },
    {
      "code": 6040,
      "name": "notSuspended",
      "msg": "Participant must be suspended before liquidation (defense-in-depth)"
    },
    {
      "code": 6041,
      "name": "notLate",
      "msg": "Participant is not late (already paid this month or no overdue contribution)"
    },
    {
      "code": 6042,
      "name": "reputationDefaulted",
      "msg": "Reputation gate: user has prior defaults; new pool joins blocked (Q-11)"
    },
    {
      "code": 6043,
      "name": "monthNotEnded",
      "msg": "Current month duration has not elapsed; slash_unpaid rejected"
    },
    {
      "code": 6044,
      "name": "nothingToSlash",
      "msg": "Participant has nothing to slash (collateral already drained)"
    }
  ],
  "types": [
    {
      "name": "bid",
      "docs": [
        "One `Bid` PDA per (pool, month, user). PDA seeds:",
        "`[BID_SEED, pool.key().as_ref(), &month.to_le_bytes(), user.key().as_ref()]`.",
        "",
        "The `init` constraint on this PDA structurally enforces INV-16 (one",
        "bid per user per month): a second `commit_bid` for the same triple",
        "fails with `AccountAlreadyInitialized` before any handler logic",
        "runs, so we don't carry an explicit \"already committed\" boolean.",
        "",
        "`commit_hash` follows spec §3 / INV-14:",
        "`sha256(bid_amount.to_le_bytes() (8) || nonce ([u8;16]) || user_pubkey (32))`.",
        "The 56-byte input is fixed-length so reveal can deterministically",
        "reconstruct it without a length prefix.",
        "",
        "Layout matches arch §3.4 with one addition: `stake_refunded` is",
        "carried locally (instead of `_reserved` padding) so step 7's",
        "`select_winner` and the future no-reveal cleanup ix can both use it",
        "as the idempotency flag for the stake side-effect."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "month",
            "type": "u8"
          },
          {
            "name": "commitHash",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "committedAt",
            "type": "i64"
          },
          {
            "name": "stakeAmount",
            "docs": [
              "1% of `pool.contribution_amount` at commit time (Q-3). Refunded",
              "to user on successful reveal, forfeit to tier reserve on",
              "no-reveal (forfeit path is step 7's concern)."
            ],
            "type": "u64"
          },
          {
            "name": "revealed",
            "type": "bool"
          },
          {
            "name": "revealedAmount",
            "type": "u64"
          },
          {
            "name": "revealedAt",
            "type": "i64"
          },
          {
            "name": "isWinner",
            "docs": [
              "Set in step 7's `select_winner`. False at commit / reveal time."
            ],
            "type": "bool"
          },
          {
            "name": "stakeRefunded",
            "docs": [
              "True after the stake has been refunded (reveal happy path) OR",
              "forfeited to reserve (step 7 cleanup). Either side-effect is",
              "idempotent thanks to this flag."
            ],
            "type": "bool"
          },
          {
            "name": "bump",
            "type": "u8"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                16
              ]
            }
          }
        ]
      }
    },
    {
      "name": "bidCommitted",
      "docs": [
        "Spec §6 + §5.1 `commit_bid`. Emitted on each successful sealed-bid",
        "commit. Indexers can rebuild the (pool, month, user) → commit_hash",
        "map from these events alone. `stake_amount` is the 1% anti-spam",
        "stake locked into the per-pool `bid_stake_vault` (Q-3)."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "month",
            "type": "u8"
          },
          {
            "name": "commitHash",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "stakeAmount",
            "type": "u64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "bidDistributed",
      "docs": [
        "Spec §6 + §5.1 `claim_winning` bid distribution (step 8). One summary",
        "event per claim per Q-17 (instead of per-recipient): indexers can",
        "reconstruct the 75/20/5 split entirely from this single record.",
        "`participant_share` is virtual — it lives in `pool.bid_credit_balance`",
        "and discounts subsequent `contribute` calls; the on-chain tokens stay",
        "in `pool_usdc_vault`."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "month",
            "type": "u8"
          },
          {
            "name": "totalBid",
            "type": "u64"
          },
          {
            "name": "participantShare",
            "type": "u64"
          },
          {
            "name": "reserveShare",
            "type": "u64"
          },
          {
            "name": "protocolShare",
            "type": "u64"
          },
          {
            "name": "bidCreditBalanceAfter",
            "type": "u64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "bidRevealed",
      "docs": [
        "Spec §6 + §5.1 `reveal_bid`. Emitted once the user opens their",
        "commitment with the matching (bid_amount, nonce). At this point the",
        "stake is refunded back to the user's USDC ATA in the same tx."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "month",
            "type": "u8"
          },
          {
            "name": "bidAmount",
            "type": "u64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "bidStakeForfeited",
      "docs": [
        "Spec §6 — emitted whenever step 7 forfeits a committed-but-unrevealed",
        "bid's stake to the tier reserve (Q-3). Per-bid; if multiple bids",
        "expire the same month, multiple events are emitted in the same tx."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "month",
            "type": "u8"
          },
          {
            "name": "stakeAmount",
            "type": "u64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "collateralRefunded",
      "docs": [
        "Collateral refunded to a participant after pool completion."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "participant",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "contribution",
      "docs": [
        "Spec §6 + §5.1 `contribute`. One emit per successful contribution.",
        "Indexers can rebuild per-month payment status from `month` +",
        "`paid_months_after`. `collateral_released` is non-zero only after the",
        "participant has won and is paying down post-win schedule (spec §4)."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "month",
            "type": "u8"
          },
          {
            "name": "amount",
            "docs": [
              "Gross USDC pulled from the user's source wallet (after applying",
              "any `bid_credit_balance` discount — Q-1)."
            ],
            "type": "u64"
          },
          {
            "name": "protocolFee",
            "type": "u64"
          },
          {
            "name": "reserveFee",
            "type": "u64"
          },
          {
            "name": "netToPool",
            "type": "u64"
          },
          {
            "name": "collateralReleased",
            "type": "u64"
          },
          {
            "name": "paidMonthsAfter",
            "type": "u16"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "defaultLiquidated",
      "docs": [
        "Emitted by `liquidate_default` for both Case A (post-win defaulter",
        "with collateral to slash) and Case B (non-winner default — zero",
        "token movement). `was_winner` lets indexers branch without rederiving",
        "`participant.has_won`. INV-1 / arch §12 solvency proof: indexers can",
        "verify the (collateral_drawn + reserve_drawn − total_owed) balance",
        "from these fields alone."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "month",
            "type": "u8"
          },
          {
            "name": "wasWinner",
            "type": "bool"
          },
          {
            "name": "totalOwed",
            "type": "u64"
          },
          {
            "name": "liquidatedFromCollateral",
            "type": "u64"
          },
          {
            "name": "drawnFromReserve",
            "type": "u64"
          },
          {
            "name": "shortfall",
            "type": "u64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "kycAttestation",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "level",
            "docs": [
              "1 = Light, 2 = Full. Never 0; the account simply does not exist",
              "when the user has no attestation."
            ],
            "type": "u8"
          },
          {
            "name": "issuedBy",
            "type": "pubkey"
          },
          {
            "name": "issuedAt",
            "type": "i64"
          },
          {
            "name": "expiresAt",
            "type": "i64"
          },
          {
            "name": "cpfHash",
            "docs": [
              "CPF hash (Brazilian tax ID). Zeroed in V1 mock; real KYC oracle",
              "will populate. // MOCK_KYC: zero placeholder."
            ],
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "sanctionsClean",
            "docs": [
              "Sanctions screen result. Always `true` in V1 mock; real KYC",
              "oracle will populate. // MOCK_KYC: always true placeholder."
            ],
            "type": "bool"
          },
          {
            "name": "bump",
            "type": "u8"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          }
        ]
      }
    },
    {
      "name": "kycAttestationIssued",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "level",
            "type": {
              "defined": {
                "name": "kycLevel"
              }
            }
          },
          {
            "name": "issuedBy",
            "type": "pubkey"
          },
          {
            "name": "issuedAt",
            "type": "i64"
          },
          {
            "name": "expiresAt",
            "type": "i64"
          },
          {
            "name": "isMock",
            "docs": [
              "`true` if the attestation was issued via the V1 `mock_issue_kyc`",
              "path; `false` for production `issue_kyc_attestation` (not yet",
              "implemented). Indexers can colour rows by this flag."
            ],
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "kycLevel",
      "docs": [
        "KYC level. Wire bytes: `None = 0`, `Light = 1`, `Full = 2`. Matches",
        "arch §3.6's u8 encoding. `None` is never written into a",
        "`KycAttestation` account (the account simply doesn't exist), but the",
        "variant is present so `UserReputation.kyc_status` can carry it."
      ],
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "none"
          },
          {
            "name": "light"
          },
          {
            "name": "full"
          }
        ]
      }
    },
    {
      "name": "latePayment",
      "docs": [
        "Emitted by `mark_late_payment` when a participant misses the strict",
        "in-window contribution boundary and lands in the day-1..=5 grace",
        "period. Single emit per (participant, month) — repeat marks revert.",
        "`accrued_penalty` is the participant's *cumulative* penalty across",
        "all months; new total after this mark."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "month",
            "type": "u8"
          },
          {
            "name": "penaltyAdded",
            "type": "u64"
          },
          {
            "name": "accruedPenalty",
            "type": "u64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "liquidationShortfall",
      "docs": [
        "Emitted only when `liquidate_default`'s reserve draw could not fully",
        "cover the shortfall. Off-chain alerting hook (arch §5.4): the protocol",
        "is technically still solvent because the missing amount is recorded",
        "here, but the deficit must be made up by future reserve top-ups."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "month",
            "type": "u8"
          },
          {
            "name": "shortfall",
            "type": "u64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "monthAdvanced",
      "docs": [
        "Spec §6 + §5.1 `advance_month`. Emitted on each successful month tick",
        "(current_month → current_month + 1). Final tick (12 → 13) emits",
        "`PoolCompleted` instead of this."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "newMonth",
            "type": "u8"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "monthWinner",
      "docs": [
        "Per-month winner record. Layout fixed by arch §3.2 (99 bytes per",
        "entry). Stored inside `Pool.winners` as `[MonthWinner; 12]` with the",
        "`month == 0` sentinel meaning \"slot not yet filled\"."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "month",
            "type": "u8"
          },
          {
            "name": "winner",
            "type": "pubkey"
          },
          {
            "name": "winningBid",
            "type": "u64"
          },
          {
            "name": "grossPayout",
            "type": "u64"
          },
          {
            "name": "netPayout",
            "type": "u64"
          },
          {
            "name": "selectedAt",
            "type": "i64"
          },
          {
            "name": "selectionMethod",
            "type": {
              "defined": {
                "name": "selectionMethod"
              }
            }
          },
          {
            "name": "claimed",
            "type": "bool"
          },
          {
            "name": "reserved",
            "docs": [
              "Reserved padding for forward compat. Sized to keep arch §3.2's",
              "99-byte total. SPEC_QUESTION-15 mitigation: kept smaller than",
              "arch §3.2's nominal 32-byte target so the surrounding `Pool`",
              "struct fits into Anchor's 4 KB-per-frame `try_accounts` budget."
            ],
            "type": {
              "array": [
                "u8",
                8
              ]
            }
          }
        ]
      }
    },
    {
      "name": "participant",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "joinedAt",
            "type": "i64"
          },
          {
            "name": "paidMonths",
            "docs": [
              "Bitmap, bit N = month N+1 paid. Bit 0 is set on `join_pool`",
              "because the join contribution covers month 1 (spec §5.1)."
            ],
            "type": "u16"
          },
          {
            "name": "hasWon",
            "type": "bool"
          },
          {
            "name": "winMonth",
            "type": "u8"
          },
          {
            "name": "bidAmountWhenWon",
            "type": "u64"
          },
          {
            "name": "collateralLocked",
            "type": "u64"
          },
          {
            "name": "collateralInitial",
            "type": "u64"
          },
          {
            "name": "isDefaulted",
            "type": "bool"
          },
          {
            "name": "isSuspended",
            "type": "bool"
          },
          {
            "name": "defaultedAt",
            "type": "i64"
          },
          {
            "name": "latePenaltyAccrued",
            "docs": [
              "200 bps (2%) penalty accrued via `mark_late_payment` (spec §4 +",
              "SPEC_QUESTION-6). Cleared when the participant cures by calling",
              "`contribute` (penalty is added on top of contribution and routed",
              "to `pool.bid_credit_balance` per Q-6) OR rolled into the",
              "liquidation amount on `liquidate_default`. Renamed from step 4's",
              "`late_penalty_accrued` for spec-§3 alignment without churning",
              "INIT_SPACE."
            ],
            "type": "u64"
          },
          {
            "name": "liquidationAmount",
            "docs": [
              "Total liquidated USDC (collateral + reserve drawdown) for this",
              "participant, populated on `liquidate_default`. Repurposed from",
              "step 4's unused `pending_credit` field — same 8-byte slot, no",
              "INIT_SPACE delta. SPEC_QUESTION-31 size playbook."
            ],
            "type": "u64"
          },
          {
            "name": "completedCyclesAtJoin",
            "docs": [
              "Snapshot of `UserReputation.pools_completed` at join (Q-7)."
            ],
            "type": "u8"
          },
          {
            "name": "bump",
            "type": "u8"
          },
          {
            "name": "collateralReleasePerMonth",
            "docs": [
              "Per-on-time-payment collateral release amount, cached at win-time",
              "(step 8 — `claim_winning`). Spec §4 collateral release schedule:",
              "`collateral_initial / months_remaining_at_win`. Step 5 reads this",
              "inside `contribute`'s post-win release branch; the field is `0`",
              "until step 8 actually populates it. SPEC_QUESTION-15 reserved-",
              "shrink: 8 bytes carved out of `_reserved` to keep total stable."
            ],
            "type": "u64"
          },
          {
            "name": "isLate",
            "docs": [
              "Step 10 default cascade — set by `mark_late_payment` (day 1 of",
              "grace). 1 byte carved from `_reserved` (24 → 23). Spec §4 + §5.1."
            ],
            "type": "bool"
          },
          {
            "name": "lateMarkedAt",
            "docs": [
              "Wall-clock when `mark_late_payment` flagged this participant. Used",
              "by `mark_late_payment` to prevent double-mark within the same",
              "month. 8 bytes carved from `_reserved` (23 → 15)."
            ],
            "type": "i64"
          },
          {
            "name": "suspendedAt",
            "docs": [
              "Wall-clock when `suspend_participant` flagged this participant.",
              "8 bytes carved from `_reserved` (15 → 7)."
            ],
            "type": "i64"
          },
          {
            "name": "reserved",
            "docs": [
              "Reserved padding for forward compat. Step 10 default-cascade",
              "fields (`is_late: 1, late_marked_at: 8, suspended_at: 8` = 17 B)",
              "were carved from the original 24 B; `liquidation_amount` was",
              "repurposed in-place from `pending_credit`. Net Participant size",
              "delta vs step 9: 0 bytes. SPEC_QUESTION-31 size playbook."
            ],
            "type": {
              "array": [
                "u8",
                7
              ]
            }
          }
        ]
      }
    },
    {
      "name": "participantJoined",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "slotIndex",
            "type": "u8"
          },
          {
            "name": "grossContribution",
            "type": "u64"
          },
          {
            "name": "protocolFee",
            "type": "u64"
          },
          {
            "name": "reserveFee",
            "type": "u64"
          },
          {
            "name": "netToPool",
            "type": "u64"
          },
          {
            "name": "completedCyclesAtJoin",
            "type": "u8"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "participantSlashed",
      "docs": [
        "Emitted by `slash_unpaid`: a participant who failed to contribute",
        "for `month` had `slash_amount` deducted from their collateral and",
        "forwarded into the yield adapter so the monthly pot stays whole.",
        "`is_defaulted_after` is true iff the slash exhausted their",
        "collateral (i.e. they're now unable to back further months)."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "month",
            "type": "u8"
          },
          {
            "name": "slashAmount",
            "type": "u64"
          },
          {
            "name": "collateralLockedAfter",
            "type": "u64"
          },
          {
            "name": "isDefaultedAfter",
            "type": "bool"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "participantSuspended",
      "docs": [
        "Emitted by `suspend_participant` once day 6 of the unpaid window",
        "elapses. From this point onward `commit_bid` rejects the user;",
        "`contribute` may still cure (Q-6), and `liquidate_default` runs at",
        "day 30+."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "month",
            "type": "u8"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "phaseSkipped",
      "docs": [
        "Admin fast-forwarded a phase via `admin_skip_phase`. Devnet only;",
        "`phase` matches the `SkippedPhase` enum (0 = bid window, 1 = reveal",
        "window, 2 = month duration)."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "month",
            "type": "u8"
          },
          {
            "name": "phase",
            "type": "u8"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "pool",
      "docs": [
        "One 12-participant, 12-month pool. ~1965 bytes including",
        "discriminator (verify via `Pool::INIT_SPACE`). SPEC_QUESTION-15:",
        "always wrap in `Box<Account<'info, Pool>>` in handlers to keep the",
        "4 KB BPF stack frame breathable.",
        "",
        "SPEC_QUESTION-8: fixed-size arrays for `participants` / `winners`."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "poolId",
            "type": "u64"
          },
          {
            "name": "creator",
            "type": "pubkey"
          },
          {
            "name": "tier",
            "type": {
              "defined": {
                "name": "tier"
              }
            }
          },
          {
            "name": "contributionAmount",
            "type": "u64"
          },
          {
            "name": "participantCount",
            "type": "u8"
          },
          {
            "name": "totalMonths",
            "type": "u8"
          },
          {
            "name": "currentMonth",
            "type": "u8"
          },
          {
            "name": "startTimestamp",
            "type": "i64"
          },
          {
            "name": "monthDurationSeconds",
            "type": "i64"
          },
          {
            "name": "bidWindowSeconds",
            "type": "i64"
          },
          {
            "name": "currentMonthStartedAt",
            "type": "i64"
          },
          {
            "name": "bidWindowEndsAt",
            "type": "i64"
          },
          {
            "name": "revealWindowEndsAt",
            "type": "i64"
          },
          {
            "name": "totalContributed",
            "type": "u64"
          },
          {
            "name": "totalDistributed",
            "type": "u64"
          },
          {
            "name": "totalCollateralLocked",
            "type": "u64"
          },
          {
            "name": "bidCreditBalance",
            "type": "u64"
          },
          {
            "name": "isComplete",
            "type": "bool"
          },
          {
            "name": "vrfInFlight",
            "type": "bool"
          },
          {
            "name": "vrfAccount",
            "type": "pubkey"
          },
          {
            "name": "poolUsdcVault",
            "type": "pubkey"
          },
          {
            "name": "collateralVault",
            "type": "pubkey"
          },
          {
            "name": "adapterState",
            "type": "pubkey"
          },
          {
            "name": "bump",
            "type": "u8"
          },
          {
            "name": "version",
            "type": "u8"
          },
          {
            "name": "completedAt",
            "docs": [
              "Set by `advance_month` when the pool transitions past month 12.",
              "0 ⇒ still active. SPEC_QUESTION-15 reserved-shrink: the 8 bytes",
              "for this field came out of the nominal `_reserved` budget so",
              "total bytes / Anchor INIT_SPACE remain stable from step 4."
            ],
            "type": "i64"
          },
          {
            "name": "paidCountForCurrentMonth",
            "docs": [
              "Number of `Participant`s that have already paid for the",
              "`current_month`. Incremented on every successful `contribute`,",
              "reset to 0 on every `advance_month` tick. Used by step 8's",
              "bid-credit pro-rata formula (SPEC_QUESTION-1): each contributing",
              "participant draws `bid_credit_balance / (POOL_SIZE - paid_count)`",
              "from the credit ledger, so the pool depletes evenly as the month",
              "progresses. 1 byte carved out of `_reserved` (8 → 7) — INIT_SPACE",
              "stays stable."
            ],
            "type": "u8"
          },
          {
            "name": "participants",
            "docs": [
              "Filled left-to-right as users `join_pool`. `Some(user)` means the",
              "slot is taken; `None` means free. SPEC_QUESTION-8."
            ],
            "type": {
              "array": [
                {
                  "option": "pubkey"
                },
                12
              ]
            }
          },
          {
            "name": "winners",
            "docs": [
              "Winner per month (1-indexed). `month == 0` ⇒ unfilled."
            ],
            "type": {
              "array": [
                {
                  "defined": {
                    "name": "monthWinner"
                  }
                },
                12
              ]
            }
          },
          {
            "name": "totalYieldDistributed",
            "docs": [
              "Cumulative yield harvested + distributed for this pool across all",
              "`distribute_yield` calls. Monotonic non-decreasing (INV \"Yield",
              "monotonic\"). For Tier 0 pools this stays at 0 forever (Tier 0",
              "generates no yield by definition — spec §5.3); Tier 1 pools",
              "accumulate realized yield here in step 12. Indexers can rebuild",
              "per-pool APY from this field + `created_at`. SPEC_QUESTION-31",
              "reserved-shrink: 8 bytes added; the previous `_reserved: [u8; 7]`",
              "is exhausted and dropped to `[u8; 0]`. Net Pool size delta: +1",
              "byte vs step 8."
            ],
            "type": "u64"
          },
          {
            "name": "reserved",
            "docs": [
              "Reserved padding for forward compat. Trimmed from arch §3.2's",
              "nominal 128 bytes for SPEC_QUESTION-15 compatibility (BPF",
              "4 KB stack budget). Step 5 carved 8 bytes out for `completed_at`;",
              "step 8 carved 1 byte out for `paid_count_for_current_month`;",
              "step 9 carved 8 bytes out for `total_yield_distributed`. The",
              "remaining slot (0 bytes) is intentional — keeps the field",
              "declared for future migrations without further INIT_SPACE growth."
            ],
            "type": {
              "array": [
                "u8",
                0
              ]
            }
          }
        ]
      }
    },
    {
      "name": "poolCompleted",
      "docs": [
        "Spec §6 + §5.1 `advance_month` final tick. Indexers can mark the pool",
        "archived from this event alone."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "totalContributed",
            "type": "u64"
          },
          {
            "name": "totalDistributed",
            "type": "u64"
          },
          {
            "name": "completedAt",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "poolCreated",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "poolId",
            "type": "u64"
          },
          {
            "name": "creator",
            "type": "pubkey"
          },
          {
            "name": "tier",
            "type": {
              "defined": {
                "name": "tier"
              }
            }
          },
          {
            "name": "contributionAmount",
            "type": "u64"
          },
          {
            "name": "monthDurationSeconds",
            "type": "i64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "poolStarted",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "startTimestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "protocolClosed",
      "docs": [
        "SPEC_QUESTION-26: emitted by `admin_close_protocol` when the admin tears",
        "down the singleton `ProtocolConfig` + `protocol_fee_vault` ahead of a",
        "re-`initialize_protocol` with a different USDC mint. Indexers should",
        "treat this as \"config rotation in progress\"; a fresh `ProtocolInitialized`",
        "will follow."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "admin",
            "type": "pubkey"
          },
          {
            "name": "protocolConfig",
            "type": "pubkey"
          },
          {
            "name": "protocolFeeVault",
            "type": "pubkey"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "protocolConfig",
      "docs": [
        "Protocol-wide configuration. Singleton; PDA derived from",
        "`[PROTOCOL_CONFIG_SEED]`. Total ≈ 171 bytes including discriminator."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "admin",
            "type": "pubkey"
          },
          {
            "name": "kycOracle",
            "docs": [
              "Authority that may issue real KYC attestations. In V1 = `admin`",
              "(placeholder; SPEC_QUESTION-26). Production rotates to a",
              "dedicated oracle key (HSM-backed Idwall integration)."
            ],
            "type": "pubkey"
          },
          {
            "name": "protocolFeeVault",
            "docs": [
              "USDC token account that receives protocol fees. Owned by the",
              "`protocol_fee_vault` PDA (seeds `[PROTOCOL_FEE_VAULT_SEED]`)."
            ],
            "type": "pubkey"
          },
          {
            "name": "usdcMint",
            "docs": [
              "Canonical USDC mint pinned at protocol init."
            ],
            "type": "pubkey"
          },
          {
            "name": "protocolFeeBps",
            "type": "u16"
          },
          {
            "name": "vaultReserveFeeBps",
            "type": "u16"
          },
          {
            "name": "defiReserveFeeBps",
            "type": "u16"
          },
          {
            "name": "paused",
            "type": "bool"
          },
          {
            "name": "bump",
            "type": "u8"
          },
          {
            "name": "version",
            "type": "u8"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                64
              ]
            }
          }
        ]
      }
    },
    {
      "name": "protocolInitialized",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "admin",
            "type": "pubkey"
          },
          {
            "name": "kycOracle",
            "type": "pubkey"
          },
          {
            "name": "usdcMint",
            "type": "pubkey"
          },
          {
            "name": "protocolFeeVault",
            "type": "pubkey"
          },
          {
            "name": "protocolFeeBps",
            "type": "u16"
          },
          {
            "name": "vaultReserveFeeBps",
            "type": "u16"
          },
          {
            "name": "defiReserveFeeBps",
            "type": "u16"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "selectionMethod",
      "docs": [
        "Selection method for `MonthWinner`. Filled in by future",
        "`select_winner` / `consume_vrf_winner` instructions. Step-4 only",
        "constructs the default value (Lottery=0) for empty winner slots."
      ],
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "lottery"
          },
          {
            "name": "bid"
          }
        ]
      }
    },
    {
      "name": "tier",
      "docs": [
        "Pool tier discriminant. Mirrors `poolver_reserve::Tier`'s wire bytes:",
        "`Vault = 0`, `DeFi = 1`. The two enums must stay aligned because the",
        "reserve seed `[RESERVE_FUND_SEED, &[tier_byte]]` is derived from",
        "whichever tier is on the `Pool`. Tests assert the alignment.",
        "",
        "Borsh discriminant assignment is source-order; do NOT reorder these",
        "variants — INV-4 (tier isolation) depends on byte stability."
      ],
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "vault"
          },
          {
            "name": "deFi"
          }
        ]
      }
    },
    {
      "name": "userReputation",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "poolsJoined",
            "type": "u32"
          },
          {
            "name": "poolsCompleted",
            "type": "u32"
          },
          {
            "name": "poolsDefaulted",
            "type": "u32"
          },
          {
            "name": "totalContributedLifetime",
            "type": "u64"
          },
          {
            "name": "totalReceivedLifetime",
            "type": "u64"
          },
          {
            "name": "kycStatus",
            "docs": [
              "0 = None, 1 = Light, 2 = Full. Mirrors `KycLevel::as_u8()`."
            ],
            "type": "u8"
          },
          {
            "name": "kycAttestation",
            "type": "pubkey"
          },
          {
            "name": "lastKycAt",
            "type": "i64"
          },
          {
            "name": "bump",
            "type": "u8"
          },
          {
            "name": "monthsMissedLifetime",
            "docs": [
              "Number of (pool, month) pairs where this user was slashed for",
              "missing the contribution. Soft signal — bumps the user into the",
              "\"yellow\" tier without flipping them to \"red\" (which is reserved",
              "for full defaults). Carved out of the original `_reserved: [u8; 32]`",
              "budget; existing on-chain accounts read 0 here, which matches the",
              "\"never been slashed\" case."
            ],
            "type": "u32"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                28
              ]
            }
          }
        ]
      }
    },
    {
      "name": "userReputationInitialized",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "user",
            "type": "pubkey"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "winnerSelected",
      "docs": [
        "Spec §6 + §5.1 `select_winner`. Emitted once per month when the",
        "(sync, V1-mocked) winner-selection ix completes. `method` discriminates",
        "the bid path (`SelectionMethod::Bid`, `winning_bid > 0`) from the",
        "lottery path (`SelectionMethod::Lottery`, `winning_bid == 0`).",
        "",
        "SPEC_QUESTION-21: in V1 the lottery branch uses a deterministic",
        "pseudo-random seed (`sha256(pool || month || slot)`) instead of a",
        "real Switchboard On-Demand VRF callback. The event shape is stable so",
        "the indexer doesn't see a schema change when production swaps in real",
        "VRF. See `select_winner.rs` for the integration point."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "month",
            "type": "u8"
          },
          {
            "name": "winner",
            "type": "pubkey"
          },
          {
            "name": "winningBid",
            "type": "u64"
          },
          {
            "name": "grossPayout",
            "type": "u64"
          },
          {
            "name": "netPayout",
            "type": "u64"
          },
          {
            "name": "method",
            "type": {
              "defined": {
                "name": "selectionMethod"
              }
            }
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "winningClaimed",
      "docs": [
        "Spec §6 + §5.1 `claim_winning` (step 8). Emitted once per month when",
        "the selected winner posts collateral and receives their net payout.",
        "Indexers can rebuild the per-month claim status from this event alone.",
        "`total_collateral_required` reflects the reputation multiplier",
        "(Q-7 snapshot) and the bid premium (`winning_bid * 2`)."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "month",
            "type": "u8"
          },
          {
            "name": "winner",
            "type": "pubkey"
          },
          {
            "name": "winningBid",
            "type": "u64"
          },
          {
            "name": "netPayout",
            "type": "u64"
          },
          {
            "name": "totalCollateralRequired",
            "type": "u64"
          },
          {
            "name": "collateralReleasePerMonth",
            "type": "u64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "yieldDistributed",
      "docs": [
        "Spec §6 + §5.1 `distribute_yield` (step 9). One summary event per",
        "distribute_yield call per SPEC_QUESTION-17 (instead of per-participant):",
        "the participant share lives in `pool.bid_credit_balance` and is",
        "consumed via `contribute`'s pro-rata draw (Q-1) — same accounting",
        "pattern as `BidDistributed`. Splits per spec §4: 70/20/10 (participants",
        "/ reserve / protocol). The participant share's tokens stay in",
        "`pool_usdc_vault` and back the `bid_credit_balance` ledger; the reserve",
        "and protocol shares are real on-chain transfers."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "totalYield",
            "type": "u64"
          },
          {
            "name": "participantShare",
            "type": "u64"
          },
          {
            "name": "reserveShare",
            "type": "u64"
          },
          {
            "name": "protocolShare",
            "type": "u64"
          },
          {
            "name": "bidCreditBalanceAfter",
            "type": "u64"
          },
          {
            "name": "totalYieldDistributedAfter",
            "type": "u64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "yieldHarvested",
      "docs": [
        "Spec §6 + §5.1 `distribute_yield` (step 9). Emitted at the START of",
        "every `distribute_yield` call — once per harvest, regardless of whether",
        "`yield_amount` is zero or positive. Tier 0 pools always emit",
        "`yield_amount = 0` (spec §5.3); Tier 1 pools (step 12) emit the",
        "realized USDC delta from the underlying DeFi adapter.",
        "",
        "`tier` is encoded as `u8` (0 = Vault, 1 = DeFi) so indexers don't need",
        "to maintain a parallel enum; matches the wire encoding in",
        "`Tier::as_u8()`."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "tier",
            "type": "u8"
          },
          {
            "name": "yieldAmount",
            "type": "u64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    }
  ]
};
