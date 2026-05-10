use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("CwaXY4fFE44fdaHr5c4A62V6pRJWDKbbFGDBPD6bHuEL");

#[program]
pub mod work_hub {
    use super::*;

    // 1. 企业发布任务并锁定资金
    pub fn create_task(ctx: Context<CreateTask>, amount: u64, task_id: u64) -> Result<()> {
        let task = &mut ctx.accounts.task_storage;
        task.employer = ctx.accounts.employer.key();
        task.amount = amount;
        task.task_id = task_id;
        task.is_completed = false;

        // 将资金从企业钱包转入合约托管账户 (Vault)
        let cpi_accounts = Transfer {
            from: ctx.accounts.employer_token_account.to_account_info(),
            to: ctx.accounts.vault_token_account.to_account_info(),
            authority: ctx.accounts.employer.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        token::transfer(CpiContext::new(cpi_program, cpi_accounts), amount)?;

        Ok(())
    }

    // 2. 企业确认验收，自动打钱并增加信誉
    pub fn complete_and_pay(ctx: Context<CompleteAndPay>) -> Result<()> {
        let task = &mut ctx.accounts.task_storage;
        let reputation = &mut ctx.accounts.user_reputation;

        require!(!task.is_completed, WorkError::AlreadyCompleted);
        
        // 关键修复：定义用于签名的 seeds
        // 注意：这里使用的是 vault_authority 的种子，因为它是 PDA 签名者
        let seeds = &[
            b"vault_authority".as_ref(),
            &[ctx.bumps.vault_authority], // 修正：引用正确的 bump 字段
        ];
        let signer = &[&seeds[..]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.vault_token_account.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.vault_authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        token::transfer(
            CpiContext::new_with_signer(cpi_program, cpi_accounts, signer),
            task.amount,
        )?;

        // 更新状态和信誉
        task.is_completed = true;
        reputation.score += 10; 

        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateTask<'info> {
    #[account(init, payer = employer, space = 8 + 32 + 8 + 8 + 1)]
    pub task_storage: Account<'info, TaskAccount>,
    #[account(mut)]
    pub employer: Signer<'info>,
    #[account(mut)]
    pub employer_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub vault_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CompleteAndPay<'info> {
    #[account(mut, has_one = employer)]
    pub task_storage: Account<'info, TaskAccount>,
    pub employer: Signer<'info>,
    #[account(mut)]
    pub user_reputation: Account<'info, ReputationAccount>,
    #[account(mut)]
    pub vault_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    
    /// 关键修复：添加 seeds 约束，使其成为 PDA 签名者，这样 ctx.bumps 才会有值
    #[account(
        seeds = [b"vault_authority"],
        bump
    )]
    pub vault_authority: SystemAccount<'info>,
    
    pub token_program: Program<'info, Token>,
}

#[account]
pub struct TaskAccount {
    pub employer: Pubkey,
    pub amount: u64,
    pub task_id: u64,
    pub is_completed: bool,
}

#[account]
pub struct ReputationAccount {
    pub score: u64,
}

#[error_code]
pub enum WorkError {
    #[msg("Task is already completed.")]
    AlreadyCompleted,
}
